#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// #[cfg(feature = "validator")]
// use validator::Validate;

use super::{File, Image, Video};

/// a animated image (usually gif) or muted looping video (gifv)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum Animated {
    /// an animated image (usually gif)
    Image {
        #[cfg_attr(feature = "serde", serde(flatten))]
        image: File<Image>,
    },

    /// muted looping video (aka gifv)
    Video {
        #[cfg_attr(feature = "serde", serde(flatten))]
        video: File<Video>,
    },
}
