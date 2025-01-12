use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use url::Url;

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
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
    pub size: Option<u64>,

    pub height: Option<u64>,
    pub width: Option<u64>,
    pub duration: Option<u64>,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize, Clone)]
pub struct MediaCreate {
    /// The original filename
    pub filename: String,

    /// A url to download this media from
    pub url: String,
    
    /// The size (in bytes)
    pub size: Option<u64>,

    /// TODO: The source url this media was downloaded from, if any
    pub source_url: Option<String>,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct MediaCreated {
	pub media_id: MediaId,
	
    /// A url to download your media to
	pub upload_url: Option<Url>,
}

use async_tempfile::TempFile;

use super::{ids::MediaId, UserId};

pub struct MediaUpload {
    pub create: MediaCreate,
	pub user_id: UserId,
	pub temp_file: TempFile,
}
