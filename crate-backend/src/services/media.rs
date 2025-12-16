use std::{
    cmp::Ordering,
    io::{Cursor, SeekFrom},
    sync::Arc,
};

use async_tempfile::TempFile;
use common::v1::types::{
    self, util::truncate::truncate_filename, Media, MediaCreate, MediaCreateSource, MediaId,
    MediaTrack, MediaTrackInfo, Mime, TrackSource, UserId,
};
use dashmap::DashMap;
use ffprobe::{MediaType, Metadata};
use futures_util::{stream::FuturesUnordered, FutureExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufWriter};
use tracing::{debug, error, info, span, trace, Instrument, Level};

use crate::{
    error::{Error, Result},
    ServerStateInner,
};

mod ffmpeg;
mod ffprobe;

pub struct ServiceMedia {
    pub state: Arc<ServerStateInner>,
    pub uploads: DashMap<MediaId, MediaUpload>,
}

pub struct MediaUpload {
    pub create: MediaCreate,
    pub user_id: UserId,
    pub temp_file: TempFile,
    pub temp_writer: BufWriter<TempFile>,
    pub current_size: u64,
    pub max_size: u64,
}

impl MediaUpload {
    pub async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let len = bytes.len() as u64;
        if self.current_size + len > self.max_size {
            return Err(Error::TooBig);
        }

        self.temp_writer.write_all(bytes).await?;
        self.current_size += len;
        Ok(())
    }

    pub async fn seek(&mut self, off: u64) -> Result<()> {
        self.temp_writer.seek(SeekFrom::Start(off)).await?;
        Ok(())
    }
}

/// web scale
// HACK: get arc to work with cursor/imagereader
// https://stackoverflow.com/a/77743548
#[derive(Debug, Clone)]
struct BigData(Arc<Vec<u8>>);

impl AsRef<[u8]> for BigData {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for BigData {
    fn from(value: Vec<u8>) -> Self {
        // not great that it creates an arc every time, but eh
        Self(Arc::new(value))
    }
}

impl ServiceMedia {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            uploads: DashMap::new(),
        }
    }

    pub async fn create_upload(
        &self,
        media_id: MediaId,
        user_id: UserId,
        create: MediaCreate,
    ) -> Result<()> {
        let temp_file = TempFile::new().await.expect("failed to create temp file!");
        let temp_writer = BufWriter::new(temp_file.open_rw().await?);
        trace!("create temp_file {:?}", temp_file.file_path());
        self.uploads.insert(
            media_id,
            MediaUpload {
                create,
                user_id,
                temp_file,
                temp_writer,
                current_size: 0,
                max_size: self.state.config.media_max_size,
            },
        );
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_metadata_and_mime(
        &self,
        file: &std::path::Path,
    ) -> Result<(Option<Metadata>, String)> {
        let meta = match ffprobe::extract(file).await {
            Ok(meta) => meta,
            Err(Error::Ffmpeg) => {
                let mime = get_mime(file).await?;
                return Ok((None, mime));
            }
            Err(err) => return Err(err),
        };
        let mut mime = get_mime(file).await?;
        // HACK: fix webm
        if !meta.has_video() {
            mime = mime.replace("video/webm", "audio/webm");
        }
        Ok((Some(meta), mime))
    }

    #[tracing::instrument(skip(self, media, meta))]
    pub async fn generate_thumbnails(
        &self,
        media: &mut Media,
        meta: &Metadata,
        path: &std::path::Path,
        mime: &Mime,
    ) -> Result<()> {
        trace!("media = {:?}", media);
        trace!("meta = {:?}", meta);
        let media_id = media.id;
        let mut fut = FuturesUnordered::new();
        if let Some(thumb) = meta.get_thumb_stream() {
            if thumb.codec_type == MediaType::Attachment {
                debug!("extract thumb attachment from container");
                let bytes = ffmpeg::extract_attachment(path, thumb.index).await?;
                let bytes = BigData::from(bytes);
                fut.push(
                    upload_extracted_thumb(self.state.clone(), bytes.clone(), media_id).boxed(),
                );
            } else if thumb.disposition.attached_pic == 1 {
                debug!("extract thumb stream from container");
                let bytes = ffmpeg::extract_stream(path, thumb.index).await?;
                let bytes = BigData::from(bytes);
                fut.push(
                    upload_extracted_thumb(self.state.clone(), bytes.clone(), media_id).boxed(),
                );
            } else if thumb.codec_type == MediaType::Video {
                debug!("generate thumb from video");
                let bytes = ffmpeg::generate_thumb(path).await?;
                let url = self.state.get_s3_url(&format!("media/{media_id}/poster"))?;
                let span_upload = span!(Level::DEBUG, "upload thumb");
                async {
                    let mut w = self
                        .state
                        .blobs
                        .writer_with(url.path())
                        .cache_control(
                            "public, max-age=604800, immutable, stale-while-revalidate=86400",
                        )
                        .content_type(mime.as_str())
                        .await?;
                    w.write(bytes).await?;
                    w.close().await?;
                    Result::Ok(())
                }
                .instrument(span_upload)
                .await?;
            } else {
                error!("no suitable thumbnail codec");
                return Ok(());
            }
        }
        while let Some(_track) = fut.next().await {}
        Ok(())
    }

    #[tracing::instrument(skip(self, up))]
    pub async fn process_upload(
        &self,
        up: MediaUpload,
        media_id: MediaId,
        user_id: UserId,
        filename: &str,
    ) -> Result<Media> {
        debug!("processing upload");
        let tmp = up.temp_file;
        let p = tmp.file_path().to_owned();
        let url = self.state.get_s3_url(&format!("media/{media_id}/file"))?;
        let services = self.state.services();
        let (meta, mime) = &services.media.get_metadata_and_mime(&p).await?;
        let mime: Mime = mime.parse()?;
        let mut media = Media {
            alt: up.create.alt.clone(),
            id: media_id,
            filename: filename.to_owned(),
            source: MediaTrack {
                mime: mime.clone(),
                info: match mime.parse() {
                    Ok(m) => match m.ty().as_str() {
                        "image" => {
                            let dims = image::image_dimensions(&p).ok();
                            MediaTrackInfo::Image(types::Image {
                                height: dims.as_ref().map(|d| d.1 as u64).unwrap_or_else(|| {
                                    meta.as_ref()
                                        .and_then(|m| m.height())
                                        .expect("all images have a height")
                                }),
                                width: dims.as_ref().map(|d| d.0 as u64).unwrap_or_else(|| {
                                    meta.as_ref()
                                        .and_then(|m| m.width())
                                        .expect("all images have a width")
                                }),
                                language: None,
                            })
                        }
                        // this is quite a bit harder than it looks...
                        "audio" | "video" => MediaTrackInfo::Mixed(types::Mixed {
                            height: meta.as_ref().and_then(|m| m.height()),
                            width: meta.as_ref().and_then(|m| m.width()),
                            duration: meta.as_ref().and_then(|m| m.duration().map(|d| d as u64)),
                            language: None,
                        }),
                        "text" => MediaTrackInfo::Text(types::Text { language: None }),
                        // "application" => MediaTrackInfo::Other,
                        _ => MediaTrackInfo::Other,
                    },
                    Err(_) => MediaTrackInfo::Other,
                },
                size: up.current_size,
                source: match up.create.source {
                    MediaCreateSource::Upload { .. } => TrackSource::Uploaded,
                    MediaCreateSource::Download { source_url, .. } => {
                        TrackSource::Downloaded { source_url }
                    }
                },
            },
        };
        debug!("finish upload for {}, mime {}", media_id, mime);
        trace!("finish upload for {} media {:?}", media_id, media);
        if let Some(meta) = &meta {
            if mime.starts_with("video/") || mime.starts_with("audio/") {
                let _ = self.generate_thumbnails(&mut media, meta, &p, &mime).await;
            }
        }
        debug!("finish generating thumbnails for {}", media_id);
        let upload_s3 = async {
            let mut f = tokio::fs::OpenOptions::new().read(true).open(&p).await?;
            let mut buf = vec![0u8; 1024 * 1024];
            let mut w = self
                .state
                .blobs
                .writer_with(url.path())
                .cache_control("public, max-age=604800, immutable, stale-while-revalidate=86400")
                .content_type(mime.as_str())
                .await?;
            while f.read(&mut buf).await? != 0 {
                w.write(buf.to_vec()).await?;
            }
            w.close().await?;
            info!("uploaded {} bytes to s3", up.current_size);
            Result::Ok(())
        };
        upload_s3.await?;
        drop(tmp);
        self.state
            .data()
            .media_insert(user_id, media.clone())
            .await?;
        Ok(media)
    }

    pub async fn import_from_url(&self, user_id: UserId, json: MediaCreate) -> Result<Media> {
        self.import_from_url_with_max_size(user_id, json, self.state.config.media_max_size)
            .await
    }

    pub async fn import_from_url_with_max_size(
        &self,
        user_id: UserId,
        json: MediaCreate,
        max_size: u64,
    ) -> Result<Media> {
        let (_filename, size, source_url) = match &json.source {
            MediaCreateSource::Upload { .. } => unreachable!(),
            MediaCreateSource::Download {
                filename,
                size,
                source_url,
            } => (filename, size, source_url),
        };

        let media_id = MediaId::new();
        self.create_upload(media_id, user_id, json.clone()).await?;

        let res = self.state.services().http.get(source_url.clone()).await?;

        match (size, res.content_length()) {
            (Some(max), Some(len)) if len > *max => return Err(Error::TooBig),
            (None, Some(len)) if len > max_size => return Err(Error::TooBig),
            _ => {}
        }

        self.import_from_response_inner(user_id, media_id, json, res, max_size)
            .await
    }

    pub async fn import_from_response(
        &self,
        user_id: UserId,
        json: MediaCreate,
        res: reqwest::Response,
        max_size: u64,
    ) -> Result<Media> {
        let media_id = MediaId::new();
        self.create_upload(media_id, user_id, json.clone()).await?;
        self.import_from_response_inner(user_id, media_id, json, res, max_size)
            .await
    }

    pub async fn import_from_response_inner(
        &self,
        user_id: UserId,
        media_id: MediaId,
        json: MediaCreate,
        res: reqwest::Response,
        max_size: u64,
    ) -> Result<Media> {
        let (filename, size, source_url) = match &json.source {
            MediaCreateSource::Upload { .. } => unreachable!(),
            MediaCreateSource::Download {
                filename,
                size,
                source_url,
            } => (filename, size, source_url),
        };

        match (size, res.content_length()) {
            (Some(max), Some(len)) if len > *max => return Err(Error::TooBig),
            (None, Some(len)) if len > max_size => return Err(Error::TooBig),
            _ => {}
        }

        let mut up = self.uploads.get_mut(&media_id).ok_or(Error::NotFound)?;

        debug!(
            "download media {} from {}, file {:?}",
            media_id,
            source_url,
            up.temp_file.file_path()
        );

        // TODO: retry downloads
        let mut bytes = res.bytes_stream();
        while let Some(chunk) = bytes.next().await {
            if let Err(err) = up.write(&chunk?).await {
                self.uploads.remove(&media_id);
                return Err(err);
            };
        }

        info!("finished stream download end_size={}", up.current_size);

        match size.map(|s| up.current_size.cmp(&s)) {
            Some(Ordering::Greater) => {
                self.uploads.remove(&media_id);
                Err(Error::TooBig)
            }
            Some(Ordering::Less) => Err(Error::BadStatic("failed to download content")),
            Some(Ordering::Equal) | None => {
                trace!("flush media");
                up.temp_writer.flush().await?;
                trace!("flushed media");
                drop(up);
                trace!("dropped upload");
                let (_, up) = self
                    .uploads
                    .remove(&media_id)
                    .expect("it was there a few milliseconds ago");
                trace!("processing upload");
                let filename = filename
                    .as_deref()
                    // TODO: try to parse name from Content-Disposition
                    .or_else(|| source_url.path_segments().and_then(|p| p.last()))
                    .map(|s| s.to_owned())
                    .unwrap_or_else(|| "unknown".to_owned());
                let media = self
                    .process_upload(up, media_id, user_id, &truncate_filename(&filename, 256))
                    .await?;
                debug!("finished processing media");
                Ok(media)
            }
        }
    }

    pub async fn import_from_bytes(
        &self,
        user_id: UserId,
        json: MediaCreate,
        bytes: bytes::Bytes,
    ) -> Result<Media> {
        let max_size = self.state.config.media_max_size;
        let filename = match &json.source {
            MediaCreateSource::Upload { filename, .. } => filename,
            MediaCreateSource::Download { .. } => unreachable!(),
        };

        if bytes.len() as u64 > max_size {
            return Err(Error::TooBig);
        }

        let media_id = MediaId::new();
        self.create_upload(media_id, user_id, json.clone()).await?;

        let mut up = self.uploads.get_mut(&media_id).ok_or(Error::NotFound)?;

        if let Err(err) = up.write(&bytes).await {
            self.uploads.remove(&media_id);
            return Err(err);
        }

        info!("finished stream download end_size={}", up.current_size);

        trace!("flush media");
        up.temp_writer.flush().await?;
        trace!("flushed media");
        drop(up);
        trace!("dropped upload");
        let (_, up) = self
            .uploads
            .remove(&media_id)
            .expect("it was there a few milliseconds ago");
        trace!("processing upload");
        let media = self
            .process_upload(up, media_id, user_id, &truncate_filename(filename, 256))
            .await?;
        debug!("finished processing media");
        Ok(media)
    }
}

#[tracing::instrument(skip(state, bytes))]
async fn upload_extracted_thumb(
    state: Arc<ServerStateInner>,
    bytes: BigData,
    media_id: MediaId,
) -> Result<Option<MediaTrack>> {
    let len = bytes.0.len();
    let span_probe = span!(Level::DEBUG, "probe thumbnail image");
    let (width, height, mime) = async {
        let cursor = Cursor::new(&bytes);
        let reader = image::ImageReader::new(cursor).with_guessed_format()?;
        let mime: Mime = reader
            .format()
            .ok_or(Error::BadStatic("failed to get mime type"))?
            .to_mime_type()
            .parse()?;
        let (width, height) = reader.into_dimensions()?;
        Result::Ok((width, height, mime))
    }
    .instrument(span_probe)
    .await?;
    let url = state.get_s3_url(&format!("media/{media_id}/poster"))?;
    let span_upload = span!(Level::DEBUG, "upload thumb");
    async {
        let mut w = state
            .blobs
            .writer_with(url.path())
            .cache_control("public, max-age=604800, immutable, stale-while-revalidate=86400")
            .content_type(mime.as_str())
            .await?;
        // HACK: extremely ugly clone
        w.write(bytes.0.to_vec()).await?;
        w.close().await?;
        Result::Ok(())
    }
    .instrument(span_upload)
    .await?;
    let track = MediaTrack {
        info: MediaTrackInfo::Thumbnail(types::Image {
            height: height.into(),
            width: width.into(),
            language: None,
        }),
        size: len as u64,
        mime,
        source: TrackSource::Extracted,
    };
    Ok(Some(track))
}

async fn get_mime(file: &std::path::Path) -> Result<String> {
    let mime = infer::get_from_path(file)?
        .map(|t| t.mime_type())
        .unwrap_or("application/octet-stream")
        .to_owned();
    Ok(mime)
}
