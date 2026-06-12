use common::v1::types::UserId;
use common::v2::types::media::Media;

use crate::prelude::*;
use crate::services::media::ServiceMedia;

/// a piece of media being uploaded
pub struct MediaUpload {
    // pub create: MediaCreate,
    // pub user_id: UserId,
    // pub temp_file: TempFile,
    // pub temp_writer: BufWriter<TempFile>,
    // pub current_size: u64,
    // pub max_size: u64,
    // pub finished_at: Instant,
    // pub processed_notify: Arc<tokio::sync::Notify>,
    // pub remote: Option<Remote>,
}

/// a piece of media on this server
pub enum MediaItem {
    Transferring {
        // ...
    },
    Processing {
        // ...
    },
    Uploaded,
    Consumed,
    Errored,
}

impl MediaUpload {
    pub(super) async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        todo!()
    }

    pub(super) async fn seek(&mut self, off: u64) -> Result<()> {
        todo!()
    }

    // /// return a future that resolves when this media upload is done
    // pub fn done(&self) -> MediaUploadDone {
    //     todo!()
    // }
}

struct UrlImport {}

impl ServiceMedia {
    // /// resolve a media reference
    // pub async fn resolve(&self, media_ref: MediaReference) -> Result<MediaItem> {
    //     todo!()
    // }

    // /// resolve a media reference, automatically importing it if it does not exist.
    // pub async fn import(&self, media_ref: MediaReference) -> Result<MediaItem> {
    //     todo!()
    // }

    // pub async fn import_from_reference(
    //     &self,
    //     user_id: UserId,
    //     media_ref: MediaReference,
    // ) -> Result<MediaV2> {
    //     todo!()
    // }

    /// import media from a multipart request's file
    pub async fn import_from_multipart(
        &self,
        user_id: UserId,
        file: MultipartFile,
    ) -> Result<Media> {
        todo!()
    }

    // pub async fn import_from_url(&self, user_id: UserId, json: MediaCreate) -> Result<MediaV2> {
    // pub async fn import_from_url_with_max_size(
    //     &self,
    //     user_id: UserId,
    //     json: MediaCreate,
    //     max_size: u64,
    // ) -> Result<MediaV2> {
    // pub async fn import_from_response(
    //     &self,
    //     user_id: UserId,
    //     json: MediaCreate,
    //     res: reqwest::Response,
    //     max_size: u64,
    //     session_id: Option<SessionId>,
    // ) -> Result<MediaV2> {
    // pub async fn import_from_response_inner(
    //     &self,
    //     user_id: UserId,
    //     media_id: MediaId,
    //     json: MediaCreate,
    //     res: reqwest::Response,
    //     max_size: u64,
    //     session_id: Option<SessionId>,
    // ) -> Result<MediaV2> {
    // pub async fn import_from_bytes(
    //     &self,
    //     user_id: UserId,
    //     json: MediaCreate,
    //     bytes: bytes::Bytes,
    // ) -> Result<MediaV2> {
    // pub fn import_multipart(&self, file: MultipartFile) {}
    // #[tracing::instrument(skip(self))]
    // pub async fn load_remote_media(
    //     &self,
    //     user_id: UserId,
    //     remote: Remote,
    //     cdn_url: Url,
    // ) -> Result<MediaV2> {

    // pub async fn create_upload(
    //     &self,
    //     media_id: MediaId,
    //     user_id: UserId,
    //     create: MediaCreate,
    //     remote: Option<Remote>,
    // ) -> Result<()> {
}
