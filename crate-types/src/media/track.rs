use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use super::Mime;

/// A unique "view" of this piece of media. Could be the source, an
/// audio/video track, a thumbnail, other metadata, etc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MediaTrack {
    /// Extra metadata about this track
    #[serde(flatten)]
    pub info: MediaTrackInfo,

    /// The url where this track may be downloaded from
    pub url: Url,

    /// The blob's length in bytes
    #[serde(flatten)]
    pub size: MediaSize,

    /// the mime type of this view
    pub mime: Mime,

    /// Where this track came from
    pub source: TrackSource,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct File {
    /// The original filename
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaPatch {
    /// Descriptive alt text, not entirely unlike a caption
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<Option<String>>,
}

// TODO: the language for this piece of media
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Language(pub String);

/// metadata for videos
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Video {
    pub height: u64,
    pub width: u64,
    pub duration: u64,
    pub codec: String,
    pub language: Option<Language>,
}

/// metadata for audio
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Audio {
    pub duration: u64,
    pub codec: String,
    pub language: Option<Language>,
}

/// metadata for images
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Image {
    pub height: u64,
    pub width: u64,
    pub language: Option<Language>,
}

/// metadata for captions/subtitles
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TimedText {
    pub duration: u64,
    pub language: Option<Language>,
}

/// metadata for text
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Text {
    pub language: Option<Language>,
}

/// multiple pieces of metadata mixed together
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Mixed {
    pub height: Option<u64>,
    pub width: Option<u64>,
    pub duration: Option<u64>,
    pub language: Option<Language>,
}

/// metadata about a particular track
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MediaTrackInfo {
    /// a video stream
    Video(Video),

    /// an audio stream
    Audio(Audio),

    /// the "main" image
    Image(Image),

    // TODO: trickplay/storyboard image
    // Trickplay(Image),
    /// thumbnails
    Thumbnail(Image),

    /// subtitles/captions
    TimedText(TimedText),

    /// text
    Text(Text),

    /// multiple types of media mixed together (eg. most videos are uploaded with video and audio streams)
    Mixed(Mixed),

    /// other or unknown media type
    Other,
}

/// Where this track came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum TrackSource {
    /// manually uploaded by the user
    Uploaded,

    /// downloaded from another url
    Downloaded { source_url: Url },

    /// extracted out of a file, without modification
    Extracted,

    /// generated from a file
    Generated,
}

// TODO: impl media streaming
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "size_unit", content = "size")]
pub enum MediaSize {
    /// if the size is known
    Bytes(u64),

    /// approximate bandwidth if the size is unknown (media streaming)
    BytesPerSecond(u64),
}

impl MediaTrack {
    pub fn approximate_bytes(&self) -> u64 {
        match self.size {
            MediaSize::Bytes(s) => s,
            MediaSize::BytesPerSecond(bps) => {
                self.info
                    .duration()
                    .expect("MediaSize::BytesPerSecond without duration is invalid")
                    * bps
            }
        }
    }
}

impl MediaTrackInfo {
    pub fn dimensions(&self) -> Option<(u64, u64)> {
        match &self {
            MediaTrackInfo::Video(video) => Some((video.width, video.height)),
            MediaTrackInfo::Image(image) => Some((image.width, image.height)),
            MediaTrackInfo::Thumbnail(image) => Some((image.width, image.height)),
            MediaTrackInfo::Mixed(mixed) => match (mixed.width, mixed.height) {
                (Some(w), Some(h)) => Some((w, h)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn duration(&self) -> Option<u64> {
        match &self {
            MediaTrackInfo::Video(video) => Some(video.duration),
            MediaTrackInfo::Audio(audio) => Some(audio.duration),
            MediaTrackInfo::TimedText(timed_text) => Some(timed_text.duration),
            MediaTrackInfo::Mixed(mixed) => mixed.duration,
            _ => None,
        }
    }

    pub fn codec(&self) -> Option<&str> {
        match &self {
            MediaTrackInfo::Video(video) => Some(video.codec.as_str()),
            MediaTrackInfo::Audio(audio) => Some(audio.codec.as_str()),
            _ => None,
        }
    }

    // TODO: avoid cloning
    pub fn language(&self) -> Option<Language> {
        match &self {
            MediaTrackInfo::Video(video) => video.language.clone(),
            MediaTrackInfo::Audio(audio) => audio.language.clone(),
            MediaTrackInfo::Image(image) => image.language.clone(),
            MediaTrackInfo::Thumbnail(image) => image.language.clone(),
            MediaTrackInfo::TimedText(timed_text) => timed_text.language.clone(),
            MediaTrackInfo::Text(text) => text.language.clone(),
            MediaTrackInfo::Mixed(mixed) => mixed.language.clone(),
            _ => None,
        }
    }
}
