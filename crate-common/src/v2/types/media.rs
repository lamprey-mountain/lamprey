#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    util::{Diff, Time},
    MediaId, Mime, UserId,
};

/// A reference to a piece of media to be used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(untagged)]
pub enum MediaReference {
    /// Use this piece of uploaded media. Prefer using this whenever possible.
    Media { media_id: MediaId },

    /// Shortcut to download media from a url. Saves a few requests for uploading.
    Url { source_url: Url },

    /// Shortcut to create media from form data. Only usable if the request body is multipart/form-data.
    Attachment { field_name: String },
}

/// request body for `media_done`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct MediaDoneParams {
    /// Whether to process this media asynchronously.
    ///
    /// If this is true, return 202 Accepted immediately and send a `MediaProcessed` event when your media is done processing.
    #[serde(default, rename = "async")]
    pub process_async: bool,
}

/// The status for this media
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaStatus {
    /// Newly created and is waiting for either
    ///
    /// - the client to begin uploading via `media_upload` route
    /// - the downlod from `source_url` to complete
    Transferring,

    /// This media is done being uploaded and is being scanned by the server.
    Processing,

    /// This media is done being uploaded and processed.
    Uploaded,

    /// This media is `Uploaded` and linked to some resource. `strip_exif` can no longer be edited. The underlying blob is now immutable and can be fetched via cdn routes.
    Consumed,
}

/// A piece of media.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Media {
    pub id: MediaId,
    pub status: MediaStatus,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub filename: String,

    /// Descriptive alt text.
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<String>,

    /// The underlying blob's length in bytes.
    pub size: u64,

    /// The mime type of this piece of media.
    pub mime: Mime,

    /// Where this piece of media was downloaded from, if it was downloaded instead of uploaded.
    pub source_url: Option<Url>,

    /// Additional filetype-specific metadata for the file
    pub metadata: MediaMetadata,

    /// The user who uploaded this media. Only exists for admins or if you uploaded this media
    pub user_id: Option<UserId>,

    /// If this media was deleted, when it was deleted. Only exists for admins.
    pub deleted_at: Option<Time>,

    /// The results of automated scans.
    pub scans: Vec<MediaScan>,

    /// Whether this media can be fetched through the `/thumb/{media_id}` cdn route.
    pub has_thumbnail: bool,

    /// Whether this media can be fetched through the `/gifv/{media_id}` cdn route.
    pub has_gifv: bool,
}

/// An automated scan result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaScan {
    /// The name of the media scanner (eg. `nsfw`, `malware`)
    pub key: String,

    /// The confidence score of the scan, from 0.0 to 1.0
    pub result: f32,

    /// The version of the scanner that was used for this attachment.
    pub version: u16,
}

/// Filetype-specific metadata
// TODO: consider using NonZeroU64 if i am sure its valid, eg. double check no image format allows image height/width zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MediaMetadata {
    /// An image file
    Image {
        /// the width of the image in pixels
        width: u64,

        /// the height of the image in pixels
        height: u64,
    },

    /// A video file
    Video {
        /// the width of the video in pixels
        width: u64,

        /// the height of the video in pixels
        height: u64,

        /// the duration of the video in seconds
        duration: u64,
    },

    /// An audio file
    Audio {
        /// the duration of the video in seconds
        duration: u64,
    },

    /// A generic file that can be previewed in a pre/code block
    Text,

    /// A generic file
    File,
}

/// An update to a piece of media
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaPatch {
    /// Descriptive alt text, not entirely unlike a caption
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<Option<String>>,

    /// The filename for this piece of media
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 256)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub filename: Option<String>,

    /// Whether to strip sensitive exif info, like location or camera make and model.
    ///
    /// This can only be changed if the media status is not `Consumed`.
    #[serde(default)]
    pub strip_exif: Option<bool>,
}

/// a request body for `media_create`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaCreate {
    /// Whether to strip sensitive exif info, like location or camera make and model.
    #[serde(default)]
    pub strip_exif: bool,

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

/// What to create this media from
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(untagged)]
pub enum MediaCreateSource {
    /// create this file by downloading it
    Download {
        /// The filename of the downloaded file; automatically detect if None
        #[cfg_attr(
            feature = "utoipa",
            schema(required = false, min_length = 1, max_length = 256)
        )]
        filename: Option<String>,

        /// The size (in bytes). HIGHLY recommended, as this lets lamprey reject oversized files earlier.
        size: Option<u64>,

        /// A url to download this media from
        source_url: Url,
    },

    /// create this file by uploading it
    Upload {
        /// The filename of this file to use
        #[cfg_attr(
            feature = "utoipa",
            schema(required = false, min_length = 1, max_length = 256)
        )]
        filename: String,

        /// The size of this file (in bytes). HIGHLY recommended, as this lets lamprey reject oversized files earlier.
        size: Option<u64>,
    },
}

/// response body for `media_create`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MediaCreated {
    /// The id of the media that has been created
    pub media_id: MediaId,

    /// A url to upload your media to. Is `None` if you used `MediaCreateSource::Download`.
    pub upload_url: Option<Url>,
}

impl MediaCreateSource {
    pub fn size(&self) -> Option<u64> {
        match self {
            MediaCreateSource::Upload { size, .. } => *size,
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

impl Diff<Media> for MediaPatch {
    fn changes(&self, other: &Media) -> bool {
        self.alt.changes(&other.alt) || self.filename.changes(&other.filename)
    }
}

use crate::v1::types::media::{Media as V1Media, MediaWithAdmin as V1MediaWithAdmin};

impl Into<V1Media> for Media {
    fn into(self) -> V1Media {
        V1Media {
            id: self.id,
            filename: self.filename,
            alt: self.alt,
            source: crate::v1::types::MediaTrack {
                info: match self.metadata {
                    MediaMetadata::Image { width, height } => {
                        crate::v1::types::MediaTrackInfo::Image(crate::v1::types::Image {
                            height,
                            width,
                            language: None,
                        })
                    }
                    MediaMetadata::Video {
                        width,
                        height,
                        duration,
                    } => crate::v1::types::MediaTrackInfo::Mixed(crate::v1::types::Mixed {
                        width: Some(width),
                        height: Some(height),
                        duration: Some(duration),
                        language: None,
                    }),
                    MediaMetadata::Audio { duration } => {
                        crate::v1::types::MediaTrackInfo::Mixed(crate::v1::types::Mixed {
                            height: None,
                            width: None,
                            duration: Some(duration),
                            language: None,
                        })
                    }
                    MediaMetadata::Text => {
                        crate::v1::types::MediaTrackInfo::Text(crate::v1::types::Text {
                            language: None,
                        })
                    }
                    MediaMetadata::File => crate::v1::types::MediaTrackInfo::Other,
                },
                size: self.size,
                mime: self.mime,
                source: if let Some(source_url) = self.source_url {
                    crate::v1::types::TrackSource::Downloaded { source_url }
                } else {
                    crate::v1::types::TrackSource::Uploaded
                },
            },
        }
    }
}

impl Into<Media> for V1Media {
    fn into(self) -> Media {
        let s = self.source;
        Media {
            id: self.id,
            // WARNING: the database is going to need to correctly populate `status`
            status: MediaStatus::Consumed,
            filename: self.filename,
            alt: self.alt,
            size: s.size,
            mime: s.mime.clone(),
            source_url: match s.source {
                crate::v1::types::TrackSource::Uploaded => None,
                crate::v1::types::TrackSource::Downloaded { source_url } => Some(source_url),
                crate::v1::types::TrackSource::Extracted => None,
                crate::v1::types::TrackSource::Generated => None,
            },
            metadata: match s.info {
                crate::v1::types::MediaTrackInfo::Video(video) => MediaMetadata::Video {
                    width: video.width,
                    height: video.height,
                    duration: video.duration,
                },
                crate::v1::types::MediaTrackInfo::Audio(audio) => MediaMetadata::Audio {
                    duration: audio.duration,
                },
                crate::v1::types::MediaTrackInfo::Image(image) => MediaMetadata::Image {
                    width: image.width,
                    height: image.height,
                },
                crate::v1::types::MediaTrackInfo::Thumbnail(image) => MediaMetadata::Image {
                    width: image.width,
                    height: image.height,
                },
                crate::v1::types::MediaTrackInfo::TimedText(_) => MediaMetadata::File,
                crate::v1::types::MediaTrackInfo::Text(_) => MediaMetadata::Text,
                crate::v1::types::MediaTrackInfo::Mixed(mixed) => match s.mime.parse() {
                    Ok(s) => match s.ty().as_str() {
                        "video" => MediaMetadata::Video {
                            width: mixed.width.unwrap_or_default(),
                            height: mixed.height.unwrap_or_default(),
                            duration: mixed.width.unwrap_or_default(),
                        },
                        "audio" => todo!(),
                        _ => MediaMetadata::File,
                    },
                    Err(_) => MediaMetadata::File,
                },
                crate::v1::types::MediaTrackInfo::Other => MediaMetadata::File,
            },
            user_id: None,
            deleted_at: None,
            scans: vec![],
            has_thumbnail: false,
            has_gifv: false,
        }
    }
}

impl Into<Media> for V1MediaWithAdmin {
    fn into(self) -> Media {
        let user_id = self.user_id;
        let deleted_at = self.deleted_at;
        Media {
            user_id: Some(user_id),
            deleted_at,
            ..self.inner.into()
        }
    }
}
