#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{util::Time, MediaId, UserId};

mod mime;
mod track;

pub use mime::Mime;
pub use track::*;

// TODO: rename to MediaV0
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Media {
    pub id: MediaId,

    /// The original filename
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub filename: String,

    /// Descriptive alt text, not entirely unlike a caption
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<String>,

    /// The source (Uploaded, Downloaded)
    pub source: MediaTrack,
}

// TODO: rename to MediaV0WithAdmin
/// media with extra metadata for admins
// maybe make this a part of media? and make each field optional
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaWithAdmin {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: Media,

    /// the user who uploaded this media
    pub user_id: UserId,

    /// if this media was deleted, and when it was deleted
    pub deleted_at: Option<Time>,
}

impl Into<Media> for MediaWithAdmin {
    fn into(self) -> Media {
        self.inner
    }
}
