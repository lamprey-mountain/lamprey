#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::unstable::types::media::{Media, MediaReference};

/// used in `message_create` and `message_update`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageAttachmentPut {
    #[serde(flatten)]
    pub media: MediaReference,

    /// Shortcut for setting alt text on the media item
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<Option<String>>,

    /// Shortcut for setting filename on the media item
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 256)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub filename: Option<String>,

    /// if this is a spoiler and should be blurred
    pub spoiler: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageAttachment {
    #[serde(flatten)]
    pub media: Media,

    /// if this is a spoiler and should be blurred
    pub spoiler: bool,
}
