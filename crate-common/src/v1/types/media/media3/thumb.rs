use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::Mime;

use super::Image;

/// a thumbnail/image preview
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Thumb {
    /// Where this file can be downloaded from
    pub url: Url,

    // keep or remove? i might not be able to know the total size beforehand if thumbnails are dynamic
    // /// File's length in bytes
    // pub size: u64,
    /// Mime type of the file
    pub mime: Mime,

    /// image metadata
    #[serde(flatten)]
    pub image: Image,
}

/// multiple sized thumbs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Thumbs(pub Vec<Thumb>);
