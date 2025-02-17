use std::{io::Cursor, process::Stdio, sync::Arc};

use async_tempfile::TempFile;
use dashmap::DashMap;
use ffprobe::{MediaType, Metadata};
use image::ImageFormat;
use tokio::{io::BufWriter, process::Command};
use tracing::{debug, error, info, trace};
use types::{
    Media, MediaCreate, MediaId, MediaSize, MediaTrack, MediaTrackInfo, TrackSource, UserId,
};

use crate::{
    error::{Error, Result},
    ServerStateInner,
};

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
            Err(Error::Ffprobe) => {
                let mime = get_mime(file).await?;
                return Ok((None, mime));
            }
            Err(err) => return Err(err),
        };
        let mut mime = get_mime(file).await?;
        // HACK: fix webm
        if meta.is_video() {
            mime = mime.replace("video/webm", "audio/webm");
        }
        Ok((Some(meta), mime))
    }

    #[tracing::instrument(skip(self))]
    pub async fn generate_thumbnails(
        &self,
        media: &mut Media,
        meta: &Metadata,
        path: &std::path::Path,
        force: bool,
    ) -> Result<()> {
        if media
            .all_tracks()
            .any(|i| matches!(i.info, MediaTrackInfo::Thumbnail(_)))
            && !force
        {
            return Ok(());
        }
        let media_id = media.id;
        let mime = get_mime(path).await?;
        let img = if mime.starts_with("image/") {
            let bytes = tokio::fs::read(path).await?;
            let cursor = Cursor::new(bytes);
            image::ImageReader::new(cursor)
                .with_guessed_format()?
                .decode()?
        } else if let Some(thumb) = meta.get_thumb_stream() {
            if thumb.codec_type == MediaType::Attachment {
                let cmd = Command::new("ffmpeg")
                    .args([
                        "-v",
                        "quiet",
                        &format!("-dump_attachment:{}", thumb.index),
                        "/dev/stdout",
                        "-y",
                        "-i",
                    ])
                    .arg(path)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .output()
                    .await?;
                let cursor = Cursor::new(cmd.stdout);
                image::ImageReader::new(cursor)
                    .with_guessed_format()?
                    .decode()?
            } else if thumb.codec_type == MediaType::Video {
                let cmd = Command::new("ffmpeg")
                    .args(["-v", "quiet", "-i"])
                    .arg(path)
                    .args([
                        "-vf",
                        "thumbnail,scale=300:300",
                        "-fames:v",
                        "1",
                        "-f",
                        "webp",
                        "-",
                    ])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .output()
                    .await?;
                let cursor = Cursor::new(cmd.stdout);
                image::ImageReader::new(cursor)
                    .with_guessed_format()?
                    .decode()?
            } else {
                error!("no suitable thumbnail codec");
                return Ok(());
            }
        } else {
            error!("no suitable thumbnail stream");
            return Ok(());
        };
        for size in [64, 320] {
            if img.width() < size && img.width() < size {
                continue;
            }
            let mut out = Cursor::new(Vec::new());
            img.thumbnail(size, size)
                .write_to(&mut out, ImageFormat::WebP)?;
            let url = format!("thumb/{media_id}/{size}");
            let len = out.get_ref().len();
            self.state
                .blobs
                .write_with(&url, out.into_inner())
                .cache_control("public, max-age=604800, immutable, stale-while-revalidate=86400")
                .await?;
            media.tracks.push(MediaTrack {
                info: MediaTrackInfo::Thumbnail(types::Image {
                    height: size as u64,
                    width: size as u64,
                    language: None,
                }),
                url,
                size: MediaSize::Bytes(len as u64),
                mime: "image/webp".to_owned(),
                source: TrackSource::Generated,
            });
        }
        Ok(())
    }

    pub async fn process_upload(
        &self,
        up: MediaUpload,
        media_id: MediaId,
        user_id: UserId,
    ) -> Result<Media> {
        let tmp = up.temp_file;
        let p = tmp.file_path().to_owned();
        let url = format!("media/{media_id}");
        let services = self.state.services();
        let (meta, mime) = &services.media.get_metadata_and_mime(&p).await?;
        let mut media = Media {
            alt: up.create.alt.clone(),
            id: media_id,
            filename: up.create.filename.clone(),
            source: MediaTrack {
                url: url.clone(),
                mime: mime.clone(),
                // TODO: use correct MediaTrackInfo type
                info: if mime.starts_with("image/") {
                    types::MediaTrackInfo::Image(types::Image {
                        height: meta
                            .as_ref()
                            .and_then(|m| m.height())
                            .expect("all images have a height"),
                        width: meta
                            .as_ref()
                            .and_then(|m| m.width())
                            .expect("all images have a width"),
                        language: None,
                    })
                } else {
                    types::MediaTrackInfo::Mixed(types::Mixed {
                        height: meta.as_ref().and_then(|m| m.height()),
                        width: meta.as_ref().and_then(|m| m.width()),
                        duration: meta.as_ref().and_then(|m| m.duration().map(|d| d as u64)),
                        language: None,
                    })
                },
                size: MediaSize::Bytes(up.create.size),
                source: TrackSource::Uploaded,
            },
            tracks: vec![],
        };
        debug!("finish upload for {}, mime {}", media_id, mime);
        if let Some(meta) = &meta {
            self.generate_thumbnails(&mut media, meta, &p, false)
                .await?;
        }
        debug!("finish generating thumbnails for {}", media_id);
        let upload_s3 = async {
            // TODO: stream upload
            let bytes = tokio::fs::read(&p).await?;
            self.state
                .blobs
                .write_with(&url, bytes)
                .cache_control("public, max-age=604800, immutable, stale-while-revalidate=86400")
                // FIXME: sometimes this fails with "failed to parse header"
                // .content_type(&mime)
                .await?;
            Result::Ok(())
        };
        upload_s3.await?;
        info!("uploaded {} bytes to s3", up.create.size);
        drop(tmp);
        self.state
            .data()
            .media_insert(user_id, media.clone())
            .await?;
        Ok(media)
    }
}

async fn get_mime(file: &std::path::Path) -> Result<String> {
    let out = Command::new("file").arg("-ib").arg(file).output().await?;
    let mime = String::from_utf8(out.stdout).expect("file has failed me");
    Ok(mime)
}
