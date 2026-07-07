use std::{collections::HashMap, io::Cursor, sync::Arc};

use async_tempfile::TempFile;
use bytes::BytesMut;
use common::{
    v1::types::{
        MessageSync, Mime,
        misc::hashes::{HashData, HashType, Hashes},
    },
    v2::types::media::{
        Media, MediaMetadata, MediaScan, MediaStatus,
        scanner::{MediaScanResponse, ScanRequest},
    },
};
use futures::stream::FuturesUnordered;
use image::ImageReader;
use lamprey_backend_core::types::media::MediaPaths;
use mediatype::MediaTypeBuf;
use sha2::{Digest, Sha512_256};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio_stream::StreamExt;
use tracing::{Instrument, Level, debug, span, trace};

use crate::{
    ServerStateInner,
    prelude::*,
    services::media::{
        ServiceMedia, ffmpeg,
        ffprobe::{self, MediaType},
        import::Upload,
        util::{Import, MediaItemState},
    },
};

/// cache control header for immutable
const IMMUTABLE: &'static str = "public, max-age=604800, immutable, stale-while-revalidate=86400";

struct MediaPipeline {
    s: Arc<ServerStateInner>,
    import: Import,
    file: TempFile,
    paths: MediaPaths,

    // cached stuff
    ffprobe_metadata: Option<Option<ffprobe::Metadata>>,
    media_metadata: Option<MediaMetadata>,
    mime: Option<MediaTypeBuf>,
    hashes: Option<Hashes>,
    poster: Option<Option<Poster>>,
}

#[derive(Debug, Clone)]
struct Poster {
    _bytes: Bytes,
}

// // TODO: split out context/cache into separate struct?
// #[derive(Default)]
// struct MediaPipelineContext {
//     ffprobe_metadata: Option<Option<ffprobe::Metadata>>,
//     media_metadata: Option<MediaMetadata>,
//     mime: Option<Mime>,
//     hashes: Option<Hashes>,
//     poster: Option<Option<Poster>>,
// }

impl MediaPipeline {
    fn from_upload(s: Arc<ServerStateInner>, import: Import, file: TempFile) -> Self {
        let paths = MediaPaths::new("media/");
        Self {
            s,
            import,
            file,
            paths,
            ffprobe_metadata: None,
            media_metadata: None,
            mime: None,
            hashes: None,
            poster: None,
        }
    }

    /// run ffprobe on this media and get the metadata
    async fn get_ffprobe_metadata(&mut self) -> Result<Option<ffprobe::Metadata>> {
        if let Some(meta) = &self.ffprobe_metadata {
            return Ok(meta.clone());
        };

        let meta = match ffprobe::extract(self.file.file_path()).await {
            Ok(meta) => Some(meta),
            Err(Error::Ffmpeg) => None,
            Err(err) => return Err(err),
        };

        self.ffprobe_metadata = Some(meta.clone());
        Ok(meta)
    }

    /// get the mime type for this piece of media
    async fn sniff_mime(&mut self) -> Result<MediaTypeBuf> {
        if let Some(mime) = &self.mime {
            return Ok(mime.clone());
        }

        let mime_str = infer::get_from_path(self.file.file_path())?
            .map(|t| t.mime_type())
            .unwrap_or("application/octet-stream");

        let mut mt: MediaTypeBuf = mime_str.parse()?;

        // HACK: fix webm
        let meta = self.get_ffprobe_metadata().await?;
        if let Some(meta) = meta {
            // PERF: dont convert to string
            if !meta.has_video() && mt.essence().to_string() == "video/webm" {
                mt = "audio/webm".parse()?;
            }
        }

        self.mime = Some(mt.clone());
        Ok(mt)
    }

    async fn get_metadata(&mut self) -> Result<MediaMetadata> {
        if let Some(meta) = &self.media_metadata {
            return Ok(meta.clone());
        };

        let mime = self.sniff_mime().await?;
        let ffmeta = self.get_ffprobe_metadata().await?;
        let meta = match mime.ty().as_str() {
            "image" => {
                let dims = image::image_dimensions(self.file.file_path()).ok();
                MediaMetadata::Image {
                    height: dims.as_ref().map(|d| d.1 as u64).unwrap_or_else(|| {
                        ffmeta
                            .as_ref()
                            .and_then(|m| m.height())
                            .expect("all images have a height")
                    }),
                    width: dims.as_ref().map(|d| d.0 as u64).unwrap_or_else(|| {
                        ffmeta
                            .as_ref()
                            .and_then(|m| m.width())
                            .expect("all images have a width")
                    }),
                }
            }
            "audio" | "video" => MediaMetadata::Video {
                height: ffmeta.as_ref().and_then(|m| m.height()).unwrap_or(0),
                width: ffmeta.as_ref().and_then(|m| m.width()).unwrap_or(0),
                duration: ffmeta
                    .as_ref()
                    .and_then(|m| m.duration().map(|d| d as u64))
                    .unwrap_or(0),
            },
            "text" => MediaMetadata::Text,
            _ => MediaMetadata::File,
        };

        self.media_metadata = Some(meta.clone());
        Ok(meta)
    }

    /// calculate all hashes for this piece of media
    async fn calculate_hashes(&mut self) -> Result<Hashes> {
        if let Some(hashes) = &self.hashes {
            return Ok(hashes.clone());
        }

        trace!("generating hashes");

        let mut hashes: HashMap<HashType, HashData> = HashMap::new();
        let file = self.file.open_ro().await?;
        let mut reader = BufReader::new(file);
        let mut buffer = [0u8; 8192];

        let mut hasher_blake3 = blake3::Hasher::new();
        let mut hasher_sha2 = Sha512_256::new();

        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            hasher_blake3.update(&buffer[..bytes_read]);
            hasher_sha2.update(&buffer[..bytes_read]);
        }

        let result = hasher_sha2.finalize();
        let hash = result.to_vec().into();
        hashes.insert(HashType::Sha512_256, hash);

        let result = hasher_blake3.finalize();
        let hash = result.as_bytes().to_vec().into();
        hashes.insert(HashType::Blake3, hash);

        let hashes: Hashes = hashes.into();
        self.hashes = Some(hashes.clone());
        Ok(hashes)
    }

    /// extract and upload the poster
    ///
    /// returns `true` if there was a poster and `false` otherwise
    async fn process_poster(&mut self) -> Result<Option<Poster>> {
        if let Some(poster) = &self.poster {
            return Ok(poster.clone());
        }

        // don't generate posters for images
        let mime = self.sniff_mime().await?;
        if mime.ty().as_str() == "image" {
            self.poster = Some(None);
            return Ok(None);
        }

        let meta = match self.get_ffprobe_metadata().await {
            Ok(Some(meta)) => meta,
            Ok(None) => return Ok(None),
            Err(err) => return Err(err),
        };

        let path = self.file.file_path();
        let bytes = if let Some(thumb) = meta.get_thumb_stream() {
            if thumb.codec_type == MediaType::Attachment {
                debug!("extract thumb attachment from container");
                ffmpeg::extract_attachment(path, thumb.index).await?
            } else if thumb.disposition.attached_pic == 1 {
                debug!("extract thumb stream from container");
                ffmpeg::extract_stream(path, thumb.index).await?
            } else if thumb.codec_type == MediaType::Video {
                debug!("generate thumb from video");
                ffmpeg::generate_thumb(path).await?
            } else {
                debug!("no suitable thumbnail codec");
                self.poster = Some(None);
                return Ok(None);
            }
        } else {
            debug!("no thumbnail for file");
            self.poster = Some(None);
            return Ok(None);
        };

        let bytes = Bytes::from(bytes);
        let url = self
            .s
            .get_s3_url(&self.paths.poster(self.import.media_id))?;

        let span_probe = span!(Level::DEBUG, "probe thumbnail image mime");
        let mime = async {
            let cursor = Cursor::new(bytes.clone());
            let reader = ImageReader::new(cursor).with_guessed_format()?;
            let mime: Mime = reader
                .format()
                .ok_or(Error::Internal("failed to get mime type".into()))?
                .to_mime_type()
                .parse()?;
            Result::Ok(mime)
        }
        .instrument(span_probe)
        .await?;

        let span_upload = span!(Level::DEBUG, "upload thumb");
        async {
            let mut w = self
                .s
                .blobs
                .writer_with(url.path())
                .cache_control(IMMUTABLE)
                .content_type(mime.as_str())
                .await?;
            w.write(bytes.clone()).await?;
            w.close().await?;
            Result::Ok(())
        }
        .instrument(span_upload)
        .await?;

        let poster = Poster { _bytes: bytes };
        self.poster = Some(Some(poster.clone()));
        Ok(Some(poster))
    }

    async fn has_thumbnail(&mut self) -> Result<bool> {
        let poster = self.process_poster().await?;
        let mime = self.sniff_mime().await?;

        let has_thumbnail = match poster {
            Some(_) => true,
            None if mime.ty().as_str() == "image" => true,
            None => false,
        };
        Ok(has_thumbnail)
    }

    /// (re)generate thumbnails for this
    async fn generate_thumbnails(&mut self) -> Result<()> {
        // NOTE: thumbnails are generally generated in crate-media, not here
        Ok(())
    }

    // async fn generate_thumbnails(&mut self) -> Result<()> {
    //     let poster = self.process_poster().await?;
    //     let mime = self.get_mime().await?;

    //     let bytes = match poster {
    //         Some(p) => todo!("generate from poster"),
    //         None if mime.ty().as_str() == "image" => todo!("generate from file itself"),
    //         None => return Ok(()),
    //     };

    //     // whether this media can have an animated thumbnail
    //     let animated = match mime.ty().as_str() {
    //         "video" => true,
    //         "image" => todo!("check if gif, apng, webp is animated"),
    //         _ => false,
    //     };

    //     let mut fut =
    //         FuturesUnordered::<Pin<Box<dyn Future<Output = Result<(u32, bool)>> + Send>>>::new();
    //     for &size in &self.s.config.media.thumb_sizes {
    //         fut.push(Box::pin(async move {
    //             // TODO: generate thumbnail
    //             Result::Ok((size, false))
    //         }));

    //         if animated {
    //             fut.push(Box::pin(async move {
    //                 // TODO: generate animated thumbnail
    //                 Result::Ok((size, true))
    //             }));
    //         }
    //     }

    //     while let Some(_) = fut.next().await {}

    //     // TODO: tracing
    //     // while let Some(res) = fut.next().await {
    //     //     match res {
    //     //         Ok((size, animated)) => debug!(?size, ?animated, "generated thumbnail"),
    //     //         Err(err) => error!(?size, ?animated, "failed to generate thumbnail"),
    //     //     }
    //     // }

    //     Ok(())
    // }

    /// (re)upload media to s3
    async fn upload(&mut self) -> Result<()> {
        let mut file = self.file.open_ro().await?;
        let mime = self.sniff_mime().await?;

        let url = self.s.get_s3_url(&self.paths.file(self.import.media_id))?;
        let mut w = self
            .s
            .blobs
            .writer_with(url.path())
            .cache_control(IMMUTABLE)
            .content_type(mime.as_str())
            .await?;

        let mut buf = BytesMut::with_capacity(1024 * 1024);
        loop {
            let n = file.read_buf(&mut buf).await?;
            if n == 0 {
                break;
            }
            w.write(buf.split_to(n).freeze()).await?;
        }
        w.close().await?;
        Ok(())
    }

    /// Scan media with all configured scanners in parallel.
    async fn scan_media(&self) -> Result<Vec<MediaScan>> {
        let scanners = &self.s.config.media.scanners;
        if scanners.is_empty() {
            return Ok(vec![]);
        }

        debug!("scanning media with {} scanners", scanners.len());

        let path = match self.file.file_path().to_str() {
            Some(p) => p.to_string(),
            None => return Ok(vec![]),
        };

        let client = &self.s.services().http.client;
        let mut futs = FuturesUnordered::new();

        for scanner in scanners {
            let scan_url = scanner.scan_url.clone();
            let key = scanner.key.clone();
            let version = scanner.version;
            let path = path.clone();
            let client = client.clone();

            futs.push(async move {
                let req = ScanRequest { path };
                let res = client.post(scan_url).json(&req).send().await.ok()?;

                let scan_res: MediaScanResponse = res.json().await.ok()?;

                Some(MediaScan {
                    key,
                    result: scan_res.score as f32,
                    version,
                })
            });
        }

        let mut scans = Vec::new();
        while let Some(result) = futs.next().await {
            if let Some(scan) = result {
                // TODO: debug!
                scans.push(scan);
            }
        }

        Ok(scans)
    }

    /// strip exif metadata from an image file
    ///
    /// returns true if the file is modified and needs to be reuploaded
    async fn strip_exif(&mut self) -> Result<bool> {
        if !self.import.strip_exif {
            return Ok(false);
        }

        let mime = self.sniff_mime().await?;
        if mime.ty().as_str() != "image" {
            return Ok(false);
        }

        let format = match mime.essence().to_string().as_str() {
            "image/jpeg" => "mjpeg",
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            // TODO: return a warning/error/something if we can't strip exif for this image
            // TODO: support avif
            _ => return Ok(false),
        };

        let path = self.file.file_path();
        let output = ffmpeg::strip_metadata(path, format).await?;

        // replace the temp file content with stripped bytes
        let mut f = self.file.open_rw().await?;
        f.set_len(0).await?;
        f.write_all(&output).await?;
        f.flush().await?;

        Ok(true)
    }
}

impl ServiceMedia {
    /// Run the media processing pipeine for an `Upload`
    #[tracing::instrument(skip(self, upload), fields(media_id = %upload.media_id()))]
    pub(super) async fn process_media(&self, upload: Upload) -> Result<()> {
        let writer = upload.writer;
        let mut pipe =
            MediaPipeline::from_upload(Arc::clone(&self.state), upload.import, upload.temp_file);

        writer.set_state(MediaItemState::Processing {
            import: pipe.import.clone(),
        });

        let _transformed = pipe.strip_exif().await?;

        let _ffprobe_metadata = pipe.get_ffprobe_metadata().await?;
        let mime = pipe.sniff_mime().await?;
        let hashes = pipe.calculate_hashes().await?;
        let scans = pipe.scan_media().await?;
        let metadata = pipe.get_metadata().await?;
        let _poster = pipe.process_poster().await?;
        let has_thumbnail = pipe.has_thumbnail().await?;

        pipe.generate_thumbnails().await?;
        pipe.upload().await?;

        let mut media: Media = (*writer.reader().media()).clone();
        media.status = MediaStatus::Uploaded;
        media.content_type = mime.as_str().parse()?;
        media.metadata = metadata;
        media.hashes = hashes;
        media.scans = scans;
        media.has_thumbnail = has_thumbnail;
        media.size = pipe.file.metadata().await?.len();

        let mut data = self.state.acquire_data().await?;
        data.media_replace(media.clone()).await?;
        data.commit().await?;

        writer.set_media(Arc::new(media.clone()));
        writer.set_ready();

        if let Some(session_id) = upload.session_id {
            self.state
                .broadcast(MessageSync::MediaProcessed { media, session_id })?;
        }

        Ok(())
    }
}
