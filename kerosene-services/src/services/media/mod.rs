use std::{sync::Arc, time::Duration};

use common::v1::types::UserId;
use common::v1::types::federation::RemoteReq;
use common::{
    v1::types::{
        MediaId,
        error::{ApiError, ErrorCode},
    },
    v2::types::media::MediaPatch,
};
use dashmap::DashMap;
use moka::future::Cache;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error};

use crate::{
    ServerStateInner,
    error::{Error, Result},
    services::media::util::MediaItemState,
};

mod ffmpeg;
mod ffprobe;
mod import;
mod process;
mod util;

pub use import::Upload;
pub use util::{Import, MediaItem};

pub struct ServiceMedia {
    state: Arc<ServerStateInner>,
    cache: Cache<MediaId, MediaItem>,
    uploads: Arc<DashMap<MediaId, Upload>>,
}

impl ServiceMedia {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache: Cache::new(1000), // TODO: make configurable
            uploads: Arc::new(DashMap::new()),
        }
    }

    pub async fn get(&self, media_id: MediaId) -> Result<MediaItem> {
        if let Some(item) = self.cache.get(&media_id).await {
            return Ok(item);
        }

        let media = self.state.data().media_select(media_id).await?;
        let writer = MediaItem::from_media(Arc::clone(&self.state), media);
        let item = writer.reader();
        self.cache.insert(media_id, item.clone()).await;
        Ok(item)
    }

    pub async fn get_remote(&self, _remote: &RemoteReq<MediaId>) -> Result<MediaItem> {
        // let media = self
        //     .state
        //     .data()
        //     .media_select_by_remote(&remote.hostname, remote.origin_id)
        //     .await?;
        // if let Some(item) = self.cache.get(&media).await {
        //     return Ok(item);
        // }

        // let item = MediaItem::from_media(media);
        // self.cache.insert(media_id, item.clone()).await;
        // Ok(item)
        todo!()
    }

    pub async fn get_many(&self, media_ids: &[MediaId]) -> Result<Vec<MediaItem>> {
        let mut items = Vec::with_capacity(media_ids.len());
        for id in media_ids {
            items.push(self.get(*id).await?);
        }
        Ok(items)
    }

    pub async fn patch(
        &self,
        user_id: UserId,
        media_id: MediaId,
        patch: MediaPatch,
    ) -> Result<MediaItem> {
        let item = self.get(media_id).await?;
        let media = item.media();
        if media.deleted_at.is_some() {
            return Err(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownMedia,
            )));
        }

        if media.user_id != Some(user_id) {
            // NOTE: should i return UnknownMedia here to prevent leaking info?
            return Err(Error::MissingPermissions);
        }

        let should_strip_exif = patch.strip_exif == Some(true);

        let mut data = self.state.acquire_data().await?;
        data.media_update(media_id, patch).await?;
        data.commit().await?;

        if should_strip_exif {
            // TODO: download, strip, reupload
        }

        // TODO: update media items
        // let writer: MediaItemWriter = todo!("somehow get writer?");
        // writer.set_media(media);

        // TODO: broadcast media update
        // let media = self.state.data().media_select(media_id).await?;
        // item.set_media(media.clone());
        // self.state.broadcast(MessageSync::MediaUpdate {
        //     media: media.clone(),
        // })?;

        Ok(item)
    }

    /// attempt to delete a piece of media
    ///
    /// only unlinked media can be deleted
    pub async fn delete(&self, _user_id: UserId, media_id: MediaId) -> Result<()> {
        // FIXME: check user_id

        if let Some(up) = self.uploads.remove(&media_id) {
            up.1.expire_handle.abort();
            return Ok(());
        }

        let links = self.state.data().media_link_select(media_id).await?;
        if links.is_empty() {
            self.state.data().media_delete(media_id).await?;
            self.cache.invalidate(&media_id).await;
            Ok(())
        } else {
            Err(Error::Conflict)
        }
    }

    /// get an upload to update it
    pub async fn upload_get(
        &self,
        media_id: MediaId,
    ) -> Option<dashmap::mapref::one::RefMut<'_, MediaId, Upload>> {
        self.bump(media_id);
        self.uploads.get_mut(&media_id)
    }

    /// finish an upload and begin processing
    pub async fn upload_done(&self, media_id: MediaId) -> Result<MediaItem> {
        if let Some((_, mut up)) = self.uploads.remove(&media_id) {
            up.expire_handle.abort();
            up.temp_writer.flush().await?;

            let item = up.writer.reader();
            let state = Arc::clone(&self.state);
            tokio::spawn(async move {
                let srv = state.services();
                if let Err(e) = srv.media.process_media(up).await {
                    error!("failed to process media {}: {}", media_id, e);
                }
            });

            Ok(item)
        } else if let Some(item) = self.cache.get(&media_id).await {
            match item.state() {
                MediaItemState::Processing { .. } | MediaItemState::Ready => Ok(item),
                _ => Err(Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownMedia,
                ))),
            }
        } else {
            Err(Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownMedia,
            )))
        }
    }

    /// reset expiration timer for an upload
    fn bump(&self, media_id: MediaId) {
        if let Some(mut up) = self.uploads.get_mut(&media_id) {
            up.expire_handle.abort();
            up.expire_handle = self.spawn_expiration_task(media_id);
        }
    }

    fn spawn_expiration_task(&self, media_id: MediaId) -> tokio::task::JoinHandle<()> {
        let uploads = Arc::clone(&self.uploads);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(300)).await;
            if uploads.remove(&media_id).is_some() {
                debug!("expired upload {}", media_id);
            }
        })
    }
}

#[cfg(any())]
pub struct ServiceMediaOld {
    // TODO: make not pub
    pub state: Arc<ServerStateInner>,
    // pub cache: Cache<MediaId, Arc<MediaItem>>, // TODO: add
    pub uploads: Arc<DashMap<MediaId, MediaUpload>>,
    pub processing: Arc<DashMap<MediaId, Arc<tokio::sync::Notify>>>, // TODO: merge into MediaItem
}

#[cfg(any())]
pub struct MediaUpload {
    pub create: MediaCreate,
    pub user_id: UserId,
    pub temp_file: TempFile,
    pub temp_writer: BufWriter<TempFile>,
    pub current_size: u64,
    pub max_size: u64,
    pub finished_at: Instant,
    pub processed_notify: Arc<tokio::sync::Notify>,
    pub remote: Option<Remote>,
}

#[cfg(any())]
impl ServiceMediaOld {
    pub async fn create_upload(
        &self,
        media_id: MediaId,
        user_id: UserId,
        create: MediaCreate,
        remote: Option<Remote>,
    ) -> Result<()> {
        let temp_file = TempFile::new().await.expect("failed to create temp file!");
        let temp_writer = BufWriter::new(temp_file.open_rw().await?);
        trace!("create temp_file {:?}", temp_file.file_path());
        let processed_notify = Arc::new(tokio::sync::Notify::new());
        self.uploads.insert(
            media_id,
            MediaUpload {
                create: create.clone(),
                user_id,
                temp_file,
                temp_writer,
                current_size: 0,
                max_size: self.state.config.media.max_size,
                finished_at: Instant::now(),
                processed_notify: Arc::clone(&processed_notify),
                remote: remote.clone(),
            },
        );
        self.processing.insert(media_id, processed_notify);

        let filename = match &create.source {
            MediaCreateSource::Upload { filename, .. } => filename.to_owned(),
            MediaCreateSource::Download { filename, .. } => {
                filename.clone().unwrap_or_else(|| "unknown".to_owned())
            }
        };

        let media = Media {
            version_id: MediaVerId::from(media_id.into_inner()),
            id: media_id,
            status: MediaStatus::Transferring,
            filename,
            alt: create.alt,
            size: 0,
            content_type: "application/octet-stream".parse().unwrap(),
            source_url: match &create.source {
                MediaCreateSource::Upload { .. } => None,
                MediaCreateSource::Download { source_url, .. } => Some(source_url.clone()),
            },
            metadata: MediaMetadata::File,
            user_id: Some(user_id),
            deleted_at: None,
            quarantine: None,
            scans: vec![],
            has_thumbnail: false,
            has_gifv: false,
            links: vec![],
            room_id: None,
            channel_id: None,
            hashes: Hashes::new(),
            strip_exif: create.strip_exif,
            remote,
        };
        self.state.data().media_insert(media).await?;

        Ok(())
    }

    #[tracing::instrument(skip(self, up))]
    async fn process_upload_inner(
        &self,
        up: MediaUpload,
        media_id: MediaId,
        user_id: UserId,
        filename: &str,
        session_id: Option<SessionId>,
    ) -> Result<Media> {
        debug!("processing upload");

        let create = up.create;
        let current_size = up.current_size;
        let tmp = up.temp_file;

        let source_url = match &create.source {
            MediaCreateSource::Upload { .. } => None,
            MediaCreateSource::Download { source_url, .. } => Some(source_url.clone()),
        };

        trace!("inserting processing status to db");
        let media_processing = Media {
            version_id: MediaVerId::new(),
            id: media_id,
            status: MediaStatus::Processing,
            filename: filename.to_owned(),
            alt: create.alt.clone(),
            size: current_size,
            content_type: "application/octet-stream".parse().unwrap(),
            source_url: source_url.clone(),
            metadata: MediaMetadata::File,
            user_id: Some(user_id),
            deleted_at: None,
            quarantine: None,
            scans: vec![],
            has_thumbnail: false,
            has_gifv: false,
            links: vec![],
            room_id: None,
            channel_id: None,
            hashes: Hashes::new(),
            strip_exif: create.strip_exif,
            remote: None,
        };
        self.state.data().media_replace(media_processing).await?;

        let p = tmp.file_path().to_owned();
        let url = self.state.get_s3_url(&format!("media/{media_id}/file"))?;
        let services = self.state.services();

        trace!("getting metadata and mime");
        let (meta, mime) = &services.media.get_metadata_and_mime(&p).await?;
        let mime: Mime = mime.parse()?;

        // scan media with configured scanners in parallel
        let scans = self.scan_media(&p).await;

        debug!("finish upload for {}, mime {}", media_id, mime);
        trace!("finish upload for {} media {:?}", media_id, media);
        if let Some(meta) = &meta {
            if mime.starts_with("video/") || mime.starts_with("audio/") {
                trace!("generating thumbnails");
                let _ = self.generate_thumbnails(media_id, meta, &p, &mime).await;
                media.has_thumbnail = true;
            }
        }
        debug!("finish generating thumbnails for {}", media_id);

        trace!("uploading to s3");
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
            loop {
                let n = f.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                w.write(buf[..n].to_vec()).await?;
            }
            w.close().await?;
            info!("uploaded {} bytes to s3", up.current_size);
            Result::Ok(())
        };
        upload_s3.await?;

        trace!("final media update in db");
        drop(tmp);
        self.state.data().media_replace(media.clone()).await?;

        if let Some(session_id) = session_id {
            let msg = MessageSync::MediaProcessed {
                media: media.clone(),
                session_id,
            };
            if let Err(e) = self.state.broadcast(msg) {
                error!("failed to broadcast MediaProcessed: {}", e);
            }
        }

        Ok(media)
    }

    pub async fn import_from_response_inner(
        &self,
        user_id: UserId,
        media_id: MediaId,
        json: MediaCreate,
        res: reqwest::Response,
        max_size: u64,
        session_id: Option<SessionId>,
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

        let mut up =
            self.uploads
                .get_mut(&media_id)
                .ok_or(Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownMedia,
                )))?;

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
                self.processing
                    .insert(media_id, up.processed_notify.clone());
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
                    .process_upload(
                        up,
                        media_id,
                        user_id,
                        &truncate_filename(&filename, 256),
                        session_id,
                    )
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
        let max_size = self.state.config.media.max_size;
        let filename = match &json.source {
            MediaCreateSource::Upload { filename, .. } => filename,
            MediaCreateSource::Download { .. } => unreachable!(),
        };

        if bytes.len() as u64 > max_size {
            return Err(Error::TooBig);
        }

        let media_id = MediaId::new();
        self.create_upload(media_id, user_id, json.clone(), None)
            .await?;

        let mut up =
            self.uploads
                .get_mut(&media_id)
                .ok_or(Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownMedia,
                )))?;

        if let Err(err) = up.write(&bytes).await {
            self.uploads.remove(&media_id);
            return Err(err);
        }

        info!("finished stream download end_size={}", up.current_size);

        trace!("flush media");
        up.temp_writer.flush().await?;
        trace!("flushed media");
        self.processing
            .insert(media_id, up.processed_notify.clone());
        drop(up);
        trace!("dropped upload");
        let (_, up) = self
            .uploads
            .remove(&media_id)
            .expect("it was there a few milliseconds ago");
        trace!("processing upload");
        let media = self
            .process_upload(
                up,
                media_id,
                user_id,
                &truncate_filename(filename, 256),
                None,
            )
            .await?;
        debug!("finished processing media");
        Ok(media)
    }

    #[tracing::instrument(skip(self))]
    pub async fn load_remote_media(
        &self,
        user_id: UserId,
        remote: Remote,
        cdn_url: Url,
    ) -> Result<Media> {
        if let Some(media) = self
            .state
            .data()
            .media_select_by_remote(&remote.hostname, remote.origin_id)
            .await?
        {
            return Ok(media);
        }

        let info = self
            .state
            .services()
            .federation
            .fetch_server_info(&remote.hostname)
            .await?;
        let url = info.cdn_url.join(&format!("/media/{}", remote.origin_id))?;

        let res = self
            .state
            .services()
            .http
            .client
            .get(url.clone())
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::BadStatic("failed to fetch remote media"));
        }

        let bytes = res.bytes().await?;

        let json = MediaCreate {
            alt: None,
            strip_exif: false,
            source: MediaCreateSource::Download {
                filename: None,
                size: Some(bytes.len() as u64),
                source_url: url,
            },
        };

        let media_id = MediaId::new();
        self.create_upload(media_id, user_id, json, Some(remote))
            .await?;

        let up = self.uploads.remove(&media_id).unwrap().1;
        self.process_upload(up, media_id, user_id, "remote_media", None)
            .await
    }

    /// Strip EXIF metadata from an image file.
    ///
    /// Downloads the image, strips EXIF data, and re-uploads it.
    #[tracing::instrument(skip(self, media_id))]
    pub async fn strip_exif(&self, media_id: MediaId) -> Result<()> {
        let media = self.state.data().media_select(media_id).await?;

        // NOTE: maybe i want to strip geolocation and other sensitive metadata from video, audio, etc?
        if !media.content_type.as_str().starts_with("image/") {
            return Ok(());
        }

        let url = self.state.get_s3_url(&format!("media/{media_id}/file"))?;
        let data = self.state.blobs.read(url.path()).await?;

        // PERF: don't do this! this clones the buffer.
        let data = data.to_vec();

        let temp_file = TempFile::new().await?;
        {
            let mut temp_writer = BufWriter::new(temp_file.open_rw().await?);
            temp_writer.write_all(&data).await?;
            temp_writer.flush().await?;
        }
        let path = temp_file.file_path();

        let format = match media.content_type.as_str() {
            "image/jpeg" => "mjpeg",
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => "image2",
        };

        // strip exif metadata here
        // this also "bakes in" rotation because ffmpeg applies it by default
        let output = ffmpeg::strip_metadata(path, format).await?;

        // reupload stripped image
        let mut w = self
            .state
            .blobs
            .writer_with(url.path())
            .cache_control("public, max-age=604800, immutable, stale-while-revalidate=86400")
            .content_type(media.content_type.as_str())
            .await?;
        w.write(output).await?;
        w.close().await?;

        info!("stripped EXIF from media {}", media_id);
        Ok(())
    }
}
