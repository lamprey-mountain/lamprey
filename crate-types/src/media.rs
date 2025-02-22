// future alternative media thing

use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{util::Diff, MediaId};

mod mime;
mod track;

pub use mime::Mime;
pub use track::*;

/// A distinct logical item of media.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// The source (Extracted, Generated)
    pub tracks: Vec<MediaTrack>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaCreate {
    /// Descriptive alt text, not entirely unlike a caption
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<String>,

    #[serde(flatten)]
    #[cfg_attr(feature = "validator", validate(nested))]
    pub source: MediaCreateSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(untagged)]
pub enum MediaCreateSource {
    Upload {
        /// The original filename
        #[cfg_attr(
            feature = "utoipa",
            schema(required = false, min_length = 1, max_length = 256)
        )]
        filename: String,

        /// The size (in bytes)
        size: u64,
    },
    Download {
        /// The original filename
        #[cfg_attr(
            feature = "utoipa",
            schema(required = false, min_length = 1, max_length = 256)
        )]
        filename: Option<String>,

        /// The size (in bytes)
        size: Option<u64>,

        /// A url to download this media from
        source_url: Url,
    },
}

#[cfg(feature = "validator")]
mod val {
    use super::MediaCreateSource;
    use serde_json::json;
    use validator::{Validate, ValidateLength, ValidationError, ValidationErrors};

    impl Validate for MediaCreateSource {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut v = ValidationErrors::new();
            if self
                .filename()
                .is_none_or(|n| n.validate_length(Some(1), Some(256), None))
            {
                Ok(())
            } else {
                let mut err = ValidationError::new("length");
                err.add_param("max".into(), &json!(256));
                err.add_param("min".into(), &json!(1));
                v.add("filename", err);
                Err(v)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MediaCreated {
    pub media_id: MediaId,

    /// A url to download your media to
    pub upload_url: Option<Url>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaPatch {
    /// Descriptive alt text, not entirely unlike a caption
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<Option<String>>,
    // TODO: editing filename
    // /// The original filename
    // #[cfg_attr(feature = "utoipa", schema(required = false, min_length = 1, max_length = 256))]
    // #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    // pub filename: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MediaRef {
    pub id: MediaId,
}

// #[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
// pub struct DeriveMedia {
//     pub title: Option<String>,
//     pub artist: Option<String>,
//     pub album: Option<String>,
//     pub comment: Option<String>,
//     pub url: Option<String>,
//     pub description: Option<String>,
//     pub date: Option<String>,
//     maybe add lyrics? location data?
// }

impl From<String> for Language {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Diff<Media> for MediaPatch {
    fn changes(&self, other: &Media) -> bool {
        self.alt.changes(&other.alt)
    }
}

impl Media {
    pub fn all_tracks(&self) -> impl Iterator<Item = &MediaTrack> {
        self.tracks.iter().chain([&self.source])
    }

    pub fn all_tracks_mut(&mut self) -> impl Iterator<Item = &mut MediaTrack> {
        self.tracks.iter_mut().chain([&mut self.source])
    }
}

impl MediaCreateSource {
    pub fn size(&self) -> Option<u64> {
        match self {
            MediaCreateSource::Upload { size, .. } => Some(*size),
            MediaCreateSource::Download { size, .. } => *size,
        }
    }

    pub fn filename(&self) -> Option<&str> {
        match self {
            MediaCreateSource::Upload { filename, .. } => Some(filename.as_str()),
            MediaCreateSource::Download { filename, .. } => filename.as_deref(),
        }
    }
}
