use std::str::FromStr;
use std::time::Instant;

use async_tempfile::TempFile;
use common::v1::types::{Mime, SessionId, UserId};
use common::v2::types::MediaId;
use common::v2::types::media::MediaMetadata;
use futures::StreamExt;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::task::JoinHandle;
use url::Url;

use crate::prelude::*;
use crate::routes::util::multipart::MultipartFile;
use crate::services::media::ServiceMedia;
use crate::services::media::util::{Import, MediaItem, MediaItemWriter};
use tracing::instrument;

// TODO: remove created media after 5 minutes, reset timer after any update
// TODO: remove uploaded media after 5 minutes

/// a piece of media being uploaded
// TODO: make fields not pub?
pub struct Upload {
    pub import: Import,

    /// the session who created this upload
    ///
    /// once the upload is done processing, a `MediaProcessed` event will be sent to this session
    pub session_id: Option<SessionId>,

    pub writer: MediaItemWriter,
    pub s: Globals,
    pub temp_file: TempFile,
    pub temp_writer: BufWriter<TempFile>,
    pub current_size: u64,

    pub finished_at: Instant,

    pub expire_handle: JoinHandle<()>,
}

impl Upload {
    pub fn media_id(&self) -> MediaId {
        self.writer.reader().media().id
    }

    pub async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let len = bytes.len() as u64;
        if self.current_size + len > self.expected_size() {
            // TODO: remove self from srv.media
            return Err(Error::TooBig);
        } else if self.current_size + len == self.expected_size() {
            // TODO: remove self from srv.media
            // TODO: begin processing
            // self.temp_writer.flush().await?;
        }

        self.temp_writer.write_all(bytes).await?;
        self.current_size += len;
        self.finished_at = Instant::now();
        Ok(())
    }

    /// get the user who created this upload
    pub fn user_id(&self) -> UserId {
        self.import.user_id
    }

    /// get the current offset into the writers
    pub fn offset(&self) -> u64 {
        self.current_size
    }

    /// the client provided file size for this media
    pub fn expected_size(&self) -> u64 {
        self.import
            .max_size
            .unwrap_or(self.s.config().media.max_size)
    }

    pub fn expects_more(&self) -> bool {
        self.offset() < self.expected_size()
    }
}

impl ServiceMedia {
    /// import media from uploaded bytes
    #[instrument(skip(self, import), fields(media_id = %import.media_id))]
    pub async fn import_from_upload(&self, import: Import) -> Result<MediaItem> {
        // TODO: return error if expected_size (import.max_size) is too big
        let media_id = import.media_id;
        let media = import.clone().to_media(
            import.filename.clone().unwrap_or_else(|| "unknown".into()),
            import.max_size.unwrap_or_default(),
            Mime::from_str("application/octet-stream").unwrap(),
            MediaMetadata::File,
        );
        let writer = MediaItem::from_media(self.state.clone(), media);
        let item = writer.reader();

        let temp_file = TempFile::new().await.expect("failed to create temp file!");
        let temp_writer = BufWriter::new(temp_file.open_rw().await?);

        let expire_handle = self.spawn_expiration_task(media_id);

        let upload = Upload {
            import,
            session_id: None,
            writer,
            s: self.state.clone(),
            temp_file,
            temp_writer,
            current_size: 0,
            finished_at: Instant::now(),
            expire_handle,
        };

        self.uploads.insert(media_id, upload);
        self.cache.insert(media_id, item.clone()).await;

        // insert initial media record into DB
        let media = item.media();
        let mut txn = self.state.begin().await?;
        txn.media_insert((*media).clone()).await?;
        txn.commit().await?;

        Ok(item)
    }

    /// import media from these bytes
    #[instrument(skip(self, import, bytes))]
    pub async fn import_from_bytes(&self, import: Import, bytes: Bytes) -> Result<MediaItem> {
        let item = self.import_from_upload(import).await?;
        let media_id = item.media().id;

        // TODO: wrap this block in tokio::spawn
        {
            let mut up = self.upload_get(media_id).await.unwrap();
            up.write(&bytes).await?;
            self.upload_done(media_id).await?;
        }

        Ok(item)
    }

    /// import media from a multipart request's file
    #[instrument(skip(self, import, file))]
    pub async fn import_from_multipart(
        &self,
        import: Import,
        file: MultipartFile,
    ) -> Result<MediaItem> {
        let bytes = file.data.clone();
        let import = import.merge_multipart(file);
        self.import_from_bytes(import, bytes).await
    }

    /// import media from this url
    #[instrument(skip(self, import, url), fields(url = %url))]
    pub async fn import_from_url(&self, import: Import, url: &Url) -> Result<MediaItem> {
        let server_max_size = self.state.config().media.max_size;
        let max_size = import.max_size;
        match max_size {
            Some(max) if max > server_max_size => return Err(Error::TooBig),
            _ => {}
        }
        // ...
        let srv = self.state.services();
        let res = srv.http.get(url.clone()).await?;
        match (max_size, res.content_length()) {
            (Some(max), Some(len)) if len > max => return Err(Error::TooBig),
            (None, Some(len)) if len > server_max_size => return Err(Error::TooBig),
            _ => {}
        }

        self.import_from_response(import, res).await
    }

    /// import media from this reqwest response
    #[instrument(skip(self, import, res))]
    pub async fn import_from_response(
        &self,
        import: Import,
        res: reqwest::Response,
    ) -> Result<MediaItem> {
        let item = self.import_from_upload(import).await?;
        let media_id = item.media().id;

        // TODO: wrap this block in tokio::spawn
        {
            let mut stream = res.bytes_stream();
            let mut up = self.upload_get(media_id).await.unwrap();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                up.write(&chunk).await?;
            }

            self.upload_done(media_id).await?;
        }

        Ok(item)
    }
}
