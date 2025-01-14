use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::MediaId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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
    pub size: u64,

    /// The height, for images and videos
    pub height: Option<u64>,

    /// The width, for images and videos
    pub width: Option<u64>,

    /// The duration in milliseconds, for audio and videos
    pub duration: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MediaCreated {
    pub media_id: MediaId,

    /// A url to download your media to
    pub upload_url: Option<Url>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MediaRef {
    pub id: MediaId,
}
