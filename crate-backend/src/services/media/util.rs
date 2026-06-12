use std::sync::Arc;

use async_tempfile::TempFile;
use common::v1::types::federation::Remote;
use common::v1::types::misc::hashes::Hashes;
use common::v1::types::{MediaId, Mime, UserId};
use common::v2::types::media::{Media, MediaCreate, MediaCreateSource, MediaMetadata, MediaStatus};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{watch, OnceCell};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::routes::util::multipart::MultipartFile;
use crate::{prelude::*, ServerStateInner};

// TODO: remove created media after 5 minutes, reset timer after any update
// TODO: remove uploaded media after 5 minutes

// NOTE: if i end up presigning media, i should do it in MediaItem

/// a piece of media on this server
#[derive(Clone)]
pub struct MediaItem {
    inner: Arc<MediaItemInner>,
    ready: watch::Receiver<bool>,
}

/// utility to edit a media item's state
pub struct MediaItemWriter {
    tx_media: watch::Sender<Arc<Media>>,
    tx_state: watch::Sender<MediaItemState>,
    // TODO: maybe use tx_state.wait_for instead and remove this field? low priority
    tx_ready: watch::Sender<bool>,
    inner: Arc<MediaItemInner>,
}

pub struct MediaItemInner {
    s: Arc<ServerStateInner>,
    media: watch::Receiver<Arc<Media>>,
    state: watch::Receiver<MediaItemState>,
    bytes: OnceCell<Bytes>,
    tempfile: OnceCell<Arc<TempFile>>,
}

/// a piece of media on this server
#[derive(Debug, Clone)]
pub enum MediaItemState {
    /// the media's bytes are being transferred to the server
    ///
    /// either the media is being uploaded by the user or the server is downloading the media from elsewhere
    Transferring { import: Import },

    /// the media is being processed
    Processing { import: Import },

    /// the media is ready to use
    Ready,

    /// an error occurred with the media
    Errored { error: MediaError },
}

#[derive(Debug, Clone)]
pub enum MediaError {
    NotFound,
    Corrupted,
    Processing,
    Timeout,

    /// unknown/other error
    // TODO: remove, i should be able to get the actual error from the Media itself
    Unknown,
}

impl From<MediaItemState> for MediaStatus {
    fn from(value: MediaItemState) -> Self {
        match value {
            MediaItemState::Transferring { .. } => MediaStatus::Transferring,
            MediaItemState::Processing { .. } => MediaStatus::Processing,
            MediaItemState::Ready => MediaStatus::Uploaded,
            MediaItemState::Errored { .. } => MediaStatus::Errored,
        }
    }
}

impl MediaItem {
    /// create a new `MediaItem` from `Media`
    pub fn from_media(s: Arc<ServerStateInner>, media: Media) -> MediaItemWriter {
        let state = match media.status {
            MediaStatus::Transferring => MediaItemState::Transferring {
                import: Import::from(media.clone()),
            },
            MediaStatus::Processing => MediaItemState::Processing {
                import: Import::from(media.clone()),
            },
            MediaStatus::Uploaded | MediaStatus::Consumed => MediaItemState::Ready,
            MediaStatus::Errored => MediaItemState::Errored {
                error: MediaError::Unknown,
            },
        };

        let (tm, rm) = watch::channel(Arc::new(media));
        let (ts, rs) = watch::channel(state);
        let (tr, rr) = watch::channel(false);

        let inner = MediaItemInner {
            media: rm,
            state: rs,
            bytes: OnceCell::new(),
            tempfile: OnceCell::new(),
            s,
        };
        let writer = MediaItemWriter {
            tx_media: tm,
            tx_state: ts,
            tx_ready: tr,
            inner: Arc::new(inner),
        };
        writer
    }

    /// get the piece of media so far
    pub fn media(&self) -> Arc<Media> {
        Arc::clone(&*self.inner.media.borrow())
    }

    pub fn state(&self) -> MediaItemState {
        self.inner.state.borrow().clone()
    }

    /// return a future that resolves when this media is ready to use
    pub async fn ready(&mut self) -> Arc<Media> {
        let _ = self.ready.wait_for(|r| r == &true).await;
        self.media()
    }

    /// download a file from the cdn to in memory bytes
    pub async fn download_bytes(&self) -> Result<Bytes> {
        self.inner
            .bytes
            .get_or_try_init(|| async move {
                // if a tempfile was already downloaded, read from it instead of refetching
                if let Some(tf) = self.inner.tempfile.get() {
                    let mut buf = Vec::new();
                    tokio::fs::File::open(tf.file_path())
                        .await?
                        .read_to_end(&mut buf)
                        .await?;
                    return Result::Ok(Bytes::from(buf));
                }

                // fetch from cdn
                let media = self.media();
                let url = self
                    .inner
                    .s
                    .get_s3_url(&format!("media/{}/file", media.id))?;
                let data = self.inner.s.blobs.read(url.path()).await?;
                Result::Ok(data.to_bytes())
            })
            .await
            .cloned()
    }

    /// download a file from the cdn to a tempfile
    pub async fn download_tempfile(&self) -> Result<Arc<TempFile>> {
        self.inner
            .tempfile
            .get_or_try_init(|| async {
                let mut file = TempFile::new().await?;
                let mut writer = file.open_rw().await?;

                // if bytes already cached, write them out instead of refetching
                if let Some(bytes) = self.inner.bytes.get() {
                    writer.write_all(bytes).await?;
                    writer.flush().await?;
                    return Result::Ok(Arc::new(file));
                }

                // fetch from cdn
                let media = self.media();
                let url = self
                    .inner
                    .s
                    .get_s3_url(&format!("media/{}/file", media.id))?;
                let mut reader = self.inner.s.blobs.reader_with(url.path()).await?;
                let mut reader = reader.into_futures_async_read(0..).await?.compat();
                tokio::io::copy(&mut reader, &mut writer).await?;
                writer.flush().await?;
                Result::Ok(Arc::new(file))
            })
            .await
            .cloned()
    }
}

impl MediaItemWriter {
    pub fn reader(&self) -> MediaItem {
        MediaItem {
            inner: Arc::clone(&self.inner),
            ready: self.tx_ready.subscribe(),
        }
    }

    pub fn set_media(&self, media: Arc<Media>) {
        let _ = self.tx_media.send(media);
    }

    pub fn set_state(&self, state: MediaItemState) {
        let _ = self.tx_state.send(state);
    }

    pub fn set_ready(&self) {
        self.set_state(MediaItemState::Ready);
        let _ = self.tx_ready.send(true);
    }
}

/// a piece of media being imported
#[derive(Debug, Clone)]
pub struct Import {
    /// the user who created this media
    pub user_id: UserId,

    /// id of this this piece of media
    pub media_id: MediaId,

    pub strip_exif: bool,
    pub alt: Option<String>,
    pub filename: Option<String>,
    pub max_size: Option<u64>,

    pub remote: Option<Remote>,
}

impl Import {
    pub fn new(user_id: UserId) -> Self {
        Self::new_with_id(MediaId::new(), user_id)
    }

    pub fn new_with_id(media_id: MediaId, user_id: UserId) -> Self {
        Self {
            user_id,
            media_id,
            strip_exif: false,
            alt: None,
            filename: None,
            max_size: None,
            remote: None,
        }
    }

    pub fn merge_multipart(mut self, file: MultipartFile) -> Self {
        if self.filename.is_none() {
            self.filename = file.filename;
        }
        self.max_size = Some(file.data.len() as u64);
        self
    }

    pub fn merge(mut self, create: MediaCreate) -> Self {
        self.strip_exif = create.strip_exif;
        self.alt = create.alt;
        match create.source {
            MediaCreateSource::Download { filename, size, .. } => {
                self.filename = filename;
                self.max_size = size;
            }
            MediaCreateSource::Upload { filename, size } => {
                self.filename = Some(filename);
                self.max_size = size;
            }
        }
        self
    }

    // most of these files *should* be ignored, but it still feels sketchy
    pub fn to_media(
        self,
        filename: String,
        size: u64,
        content_type: Mime,
        metadata: MediaMetadata,
    ) -> Media {
        Media {
            id: self.media_id,
            version_id: (*self.media_id).into(),
            status: MediaStatus::Processing,
            filename,
            size,
            content_type,
            metadata,
            alt: self.alt,
            strip_exif: self.strip_exif,
            user_id: Some(self.user_id),
            remote: self.remote,

            source_url: None,
            deleted_at: None,
            quarantine: None,
            scans: vec![],
            has_thumbnail: false,
            has_gifv: false,
            links: vec![],
            room_id: None,
            channel_id: None,
            hashes: Hashes::new(),
        }
    }
}

impl From<Media> for Import {
    fn from(value: Media) -> Self {
        Self {
            user_id: value.user_id.unwrap_or_default(),
            media_id: value.id,
            strip_exif: value.strip_exif,
            alt: value.alt,
            filename: Some(value.filename),
            max_size: Some(value.size),
            remote: value.remote,
        }
    }
}

/// media path calculator
// TODO: move to crate-backend-core
pub struct MediaPaths {
    pub(super) prefix: String,
}

impl MediaPaths {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    /// get the path for the file itself
    pub fn file(&self, media_id: MediaId) -> String {
        format!("{}/file", self.base(media_id))
    }

    /// get the path for the file's embedded thumbnail
    pub fn poster(&self, media_id: MediaId) -> String {
        format!("{}/poster", self.base(media_id))
    }

    pub fn thumb(&self, media_id: MediaId, size: u64, ext: &str) -> String {
        format!("{}/thumb/{}x{}.{}", self.base(media_id), size, size, ext)
    }

    pub fn thumb_static(&self, media_id: MediaId, size: u64, ext: &str) -> String {
        format!(
            "{}/thumb/{}x{}_static.{}",
            self.base(media_id),
            size,
            size,
            ext
        )
    }

    fn base(&self, media_id: MediaId) -> String {
        format!("{}{}", self.prefix, media_id)
    }

    // TODO: stream
    // TODO: trickplay
}
