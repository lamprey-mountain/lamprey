use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type)]
pub struct Media {
    pub id: MediaId,

    /// The original filename
    pub filename: String,

    /// A url to download this media from
    pub url: String,

    /// The source url this media was downloaded from, if any
    pub source_url: Option<String>,

    /// TODO: A url for a thumbnail, currently always null
    pub thumbnail_url: Option<String>,

    /// The mime type (file type)
    pub mime: String,

    /// Descriptive alt text, not entirely unlike a caption
    pub alt: Option<String>,

    /// The size (in bytes)
    #[sqlx(try_from = "i64")]
    // FIXME: use unsigned integers instead of signed integers
    pub size: i64,

    #[sqlx(try_from = "i64")]
    // FIXME: use unsigned integers instead of signed integers
    pub height: Option<i64>,
    
    #[sqlx(try_from = "i64")]
    // FIXME: use unsigned integers instead of signed integers
    pub width: Option<i64>,
    
    #[sqlx(try_from = "i64")]
    // FIXME: use unsigned integers instead of signed integers
    pub duration: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct MediaCreate {
    /// The original filename
    pub filename: String,

    /// Descriptive alt text, not entirely unlike a caption
    pub alt: Option<String>,

    /// A url to download this media from
    pub url: Option<Url>,
    
    /// The size (in bytes)
    pub size: u64,

    /// TODO: The source url this media was downloaded from, if any
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct MediaCreated {
	pub media_id: MediaId,
	
    /// A url to download your media to
	pub upload_url: Option<Url>,
}

use async_tempfile::TempFile;
use uuid::Uuid;

use super::{ids::MediaId, UserId};

pub struct MediaUpload {
    pub create: MediaCreate,
	pub user_id: UserId,
	pub temp_file: TempFile,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct MediaRef {
    pub id: MediaId,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "message_link_type")]
pub enum MediaLinkType {
	Message,
	MessageVersion,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct MediaLink {
	pub media_id: MediaId,
	pub target_id: Uuid,
	pub link_type: MediaLinkType,
}
