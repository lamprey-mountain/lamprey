pub use crate::v1::types::media::{
    MediaCreate,
    MediaCreateSource,
    MediaPatch,
    // MediaSize, MediaTrack,
    // MediaTrackInfo, Mime, TrackSource, ,
    Mime,
};
pub use crate::v1::types::*;

pub mod media;

pub use media::{Audio, Image, Media, MediaAny, MediaFile, Text, TimedText, Video};

/// random ideas for the future
/// most probably won't be implemented, but i'll keep track of them anyways
#[doc(hidden)]
#[allow(unused)]
mod ideas;
