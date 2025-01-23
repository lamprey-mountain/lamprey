// future alternative media thing

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use url::Url;

/// a piece of media
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Media {
    pub id: MediaId,

    /// The original filename
    pub filename: String,

    /// Descriptive alt text, not entirely unlike a caption
    pub alt: Option<String>,

    pub tracks: Vec<MediaTrack>,
}

/// metadata for videos
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Video {
    pub height: u64,
    pub width: u64,
    pub duration: u64,
    pub codec: String,
    pub language: Option<String>,
}

/// metadata for audio
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Audio {
    pub duration: u64,
    pub codec: String,
    pub language: Option<String>,
}

/// metadata for images
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Image {
    pub height: u64,
    pub width: u64,
    pub codec: String,
    pub language: Option<String>,
}

/// metadata for captions/subtitles
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct TimedText {
    pub duration: u64,
    pub language: Option<String>,
}

/// metadata for text
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Text {
    pub language: Option<String>,
}

/// metadata about a particular track
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub enum MediaTrackInfo {
    Video(Video),
    Audio(Audio),
    Image(Image),
    Trickplay(Image),
    Thumbnail(Image),
    TimedText(TimedText),
    Text(Text),
    ArbitraryMetadata(serde_json::Value),
    Other,
}

/// where this track came from
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub enum TrackSource {
    /// manually uploaded by the user
    Uploaded,
    
    /// downloaded from another url
    Downloaded {
        source_url: Url,
    },

    /// extracted out of a file, without modification
    Extracted,
    
    /// generated from a file
    Generated,
}

/// a unique "view" of this piece of media. could be the source, an audio/video track, a thumbnail, other metadata, etc
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct MediaTrack {
    /// extra metadata about this view
    pub info: MediaTrackInfo,
    
    /// the url where this view may be downloaded from
    pub url: Option<String>,
    
    /// The blob's length in bytes
    pub size: u64,
    
    /// the mime type of this view
    pub mime: String,
    
    pub source: TrackSource,
}

// struct UrlEmbed {
//     url: String,
//     title: Option<String>,
//     description: Option<String>,
//     site_name: Option<String>,
//     color: Option<String>,
//     media: Vec<Media>,
//     iframe: Option<String>,
//     author_url: Option<String>,
//     author_name: Option<String>,
//     author_avatar: Option<Media>,
// }
