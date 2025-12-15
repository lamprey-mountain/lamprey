//! files

use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{text::Language, Mime};

use super::thumb::Thumbs;

/// Represents metadata about a single file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct File<T> {
    /// The original filename
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub filename: String,

    /// File's length in bytes
    pub size: u64,

    /// Mime type of the file
    pub mime: Mime,

    /// Where this file can be downloaded from
    pub url: Url,

    /// Where this file was downloaded from, if it was downloaded instead of uploaded
    pub source_url: Option<Url>,

    /// Thumbnails for this file
    pub thumbs: Option<Thumbs>,

    /// metadata about this file
    #[serde(flatten)]
    pub meta: T,
}

/// metadata for text
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Text {
    pub language: Option<Language>,
}

/// metadata for captions/subtitles
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TimedText {
    pub duration: u64,
    pub language: Option<Language>,
}

/// metadata for videos
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Video {
    pub height: u64,
    pub width: u64,
    pub duration: u64,
}

/// metadata for audio
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Audio {
    pub duration: u64,
}

/// metadata for images
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Image {
    pub height: u64,
    pub width: u64,
}

/// a generic file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Generic {
    // (intentionally left blank)
}

pub type FileImage = File<Image>;
pub type FileVideo = File<Video>;
pub type FileAudio = File<Audio>;
pub type FileText = File<Text>;
pub type FileTimedText = File<TimedText>;
pub type FileGeneric = File<Generic>;
