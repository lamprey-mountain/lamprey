//! streamable media

use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::Mime;

use super::{Audio, Image, TimedText, Video};

/// a piece of media which can be streamed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Streamable {
    /// available tracks
    pub tracks: Vec<Track>,
    pub metadata: Metadata,
}

/// metadata about the current media
// TODO: strongly type (title, artist, album, etc)
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Metadata(pub serde_json::Value);

/// a track
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum Track {
    /// lots of thumbnails, used for scrubbing through media
    Trickplay {
        mime: Mime,
        image: Image,

        /// number of thumbnails along the x axis
        num_w: u32,

        /// number of thumbnails along the y axis
        num_h: u32,
    },

    /// a thumbnail
    Thumbnail { mime: Mime, image: Image },

    /// a transcription of the audio
    Captions { mime: Mime, text: TimedText },

    /// a description of the media
    Subtitles { mime: Mime, text: TimedText },

    /// visual stream
    Video {
        video: Video,
        bandwidth: u32,
        codec: Codec,
        url: Url,
    },

    /// auditory stream
    Audio {
        audio: Audio,
        bandwidth: u32,
        codec: Codec,
        url: Url,
    },
}

/// an entire video or audio stream
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Playlist {
    pub duration: u32,
    pub segments: Vec<Segment>,
}

/// a single fragment of a video or audio stream
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Segment {
    pub duration: u32,
    pub size: u32,
    pub url: Url,
}

// TODO: make sure this does what i think it does
/// a codec
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(untagged)]
pub enum Codec {
    /// video
    H263V2,

    /// video; aka avc
    H264,

    /// video; aka hevc
    H265,

    /// video
    Vp9,

    /// video
    Av1,

    /// audio
    Aac,

    /// audio
    Mp3,

    /// audio
    Opus,

    /// audio
    Vorbis,

    /// unknown codec
    Other(String),
}
