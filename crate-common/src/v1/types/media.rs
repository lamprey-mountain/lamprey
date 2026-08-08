use crate::v1::types::{MediaId, UserId, util::Time};
use lamprey_macros::record;

mod mime;
mod track;

pub use mime::Mime;
pub use track::*;

#[record]
#[derive(PartialEq, Eq)]
pub struct MediaV0 {
    pub id: MediaId,

    /// The original filename
    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    pub filename: String,

    /// Descriptive alt text, not entirely unlike a caption
    #[schema(required = false, min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    pub alt: Option<String>,

    /// The source (Uploaded, Downloaded)
    pub source: MediaTrack,
}

/// media with extra metadata for admins
#[record]
#[derive(PartialEq, Eq)]
pub struct MediaV0WithAdmin {
    #[serde(flatten)]
    pub inner: MediaV0,

    /// the user who uploaded this media
    pub user_id: UserId,

    /// if this media was deleted, and when it was deleted
    pub deleted_at: Option<Time>,
}

impl From<MediaV0WithAdmin> for MediaV0 {
    fn from(val: MediaV0WithAdmin) -> Self {
        val.inner
    }
}
