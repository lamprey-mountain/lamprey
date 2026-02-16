//! types used in the media proxy

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

// NOTE: theres probably a better way to define this struct
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct StreamQuery {
    /// segment index
    pub n: usize,

    /// stream identifier
    pub s: u64,
}

// TODO: move to common?
/// an available stream format
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum StreamKind {
    Video,
    Audio,
}
