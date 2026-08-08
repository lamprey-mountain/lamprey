//! types used in the media proxy

use lamprey_macros::record;

#[cfg(feature = "utoipa")]
use utoipa::IntoParams;

#[record]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(IntoParams))]
pub struct MediaQuery {
    /// if this media is still being uploaded, downloaded, or processed, block
    /// until its complete.
    ///
    /// otherwise, immediately return a 409 status code. (409 is used so that
    /// its possible to differentiate between "media doesnt exist" and "media is
    /// still being processed")
    #[serde(default = "default_true")]
    pub wait: bool,
}

#[record]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(IntoParams))]
pub struct ThumbQuery {
    /// if None, fetch the original thumbnail (eg. a video may have an embedded thumbnail)
    pub size: Option<u32>,

    /// whether to allow animated thumbnails
    #[serde(default = "default_true")]
    pub animate: bool,
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

// NOTE: theres probably a better way to define this struct
#[record]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(IntoParams))]
pub struct TrickplayQuery {
    /// number of thumbnails on the y axis
    pub height: Option<u32>,

    /// number of thumbnails on the x axis
    pub width: Option<u32>,

    /// height for each thumbnail
    pub thumb_height: Option<u32>,

    /// width for each thumbnail
    pub thumb_width: Option<u32>,
}

#[record]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(IntoParams))]
pub struct StreamQuery {
    /// segment index
    pub n: usize,

    /// stream identifier
    pub s: u64,
}

/// an available stream format
#[record]
#[derive(PartialEq, Eq)]
pub struct StreamFormat {
    pub id: u64,

    pub kind: StreamKind,
    pub codec: String,

    pub width: Option<u64>,     // video only
    pub height: Option<u64>,    // video only
    pub framerate: Option<u64>, // video only

    pub bitrate: Option<u64>,  // audio only
    pub channels: Option<u64>, // audio only
}

#[record]
#[derive(PartialEq, Eq)]
pub enum StreamKind {
    Video,
    Audio,
}
