use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    ChannelId, EmbedId, MediaId, MediaVerId, MessageId, MessageVerId, Mime, RedexId, RedexVerId,
    RoomId, UserId, federation::Remote, misc::hashes::Hashes, util::Time,
};

pub mod proxy;
pub mod scanner;

/// A reference to a piece of media to be used.
// TODO: use this in more FooCreate and FooPatch structs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(untagged))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaReference {
    /// Use this piece of uploaded media. Prefer using this whenever possible.
    Media { media_id: MediaId },

    /// Shortcut to download media from a url. Saves a few requests for uploading.
    Url { source_url: Url },

    /// Shortcut to create media from form data. Only usable if the request body is multipart/form-data.
    Attachment { media_index: u64 },
}

/// request body for `media_done`
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct MediaDoneParams {
    /// Whether to process this media asynchronously.
    ///
    /// If this is true, return 202 Accepted immediately and send a `MediaProcessed` event when your media is done processing.
    #[cfg_attr(feature = "serde", serde(default, rename = "async"))]
    pub process_async: bool,
}

// TODO: remove
/// request body for `media_upload_direct`
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MediaDirectParams {
    /// Descriptive alt text, not entirely unlike a caption
    pub alt: Option<String>,

    /// The filename for this piece of media
    pub filename: Option<String>,

    /// Whether to strip sensitive exif info, like location or camera make and model.
    #[cfg_attr(feature = "serde", serde(default))]
    pub strip_exif: bool,

    /// Whether to process this media asynchronously.
    #[cfg_attr(feature = "serde", serde(default, rename = "async"))]
    pub process_async: bool,
}

/// The status for this media
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaStatus {
    /// Newly created and is waiting for either
    ///
    /// - the client to begin uploading via `media_upload` route
    /// - the download from `source_url` to complete
    /// - the automatic download for an embed to complete
    // NOTE: what do i do if an embed has an unexpected type? eg. the service
    // says it's an image but it's actually a video? or it returns 404?
    Transferring,

    /// This media is done being uploaded and is being scanned by the server.
    Processing,

    /// This media is done being uploaded and processed.
    Uploaded,

    /// This media is `Uploaded` and linked to some resource. `strip_exif` can no longer be edited. The underlying blob is now immutable and can be fetched via cdn routes.
    Consumed,

    /// This piece of media has errored
    Errored,
}

/// A piece of media.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Media {
    pub id: MediaId,
    pub version_id: MediaVerId,
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
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub alt: Option<String>,

    /// The underlying blob's length in bytes.
    pub size: u64,

    /// The mime type of this piece of media.
    pub content_type: Mime,

    /// Where this piece of media was downloaded from, if it was downloaded instead of uploaded.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub source_url: Option<Url>,

    /// Additional filetype-specific metadata for the file
    pub metadata: MediaMetadata,

    /// The user who uploaded this media. Only exists for admins or if you uploaded this media
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub user_id: Option<UserId>,

    /// If this media was deleted, when it was deleted. Only exists for admins.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub deleted_at: Option<Time>,

    /// If this media is quarantined, this contains information about the quarantine. Only exists for admins.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub quarantine: Option<MediaQuarantine>,

    /// The results of automated scans.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub scans: Vec<MediaScan>,
    // pub ratings: ContentRatings,
    /// Whether this media can be fetched through the `/thumb/{media_id}` cdn route.
    pub has_thumbnail: bool,

    /// Whether this media can be fetched through the `/gifv/{media_id}` cdn route.
    pub has_gifv: bool,

    /// what this piece of media is linked to (admin only)
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub links: Vec<MediaLinkType>,

    // TODO: merge into links?
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub room_id: Option<RoomId>,

    // TODO: merge into links?
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub channel_id: Option<ChannelId>,

    /// the hashes of this file
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Hashes::is_empty")
    )]
    pub hashes: Hashes,

    // TODO: don't return this in the actual Media struct, just store and handle it internally
    /// Whether sensitive exif info has been stripped from this media.
    ///
    /// Once set to `true`, this cannot be unset.
    #[cfg_attr(feature = "serde", serde(default))]
    pub strip_exif: bool,

    /// if this media exists on a remote server
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub remote: Option<Remote<MediaId>>,
    // TODO: add
    // /// If this media will expire, data about its expiry.
    // #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    // pub expiry: Option<MediaExpiry>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaExpiry {
    /// when this media expires at
    pub expires_at: Time,

    /// whether this media has expired
    // NOTE: move to MediaStatus::Expired?
    pub expired: bool,
}

// /// minimal struct to represent an image
// pub struct MediaImageMinimal {
//     pub id: MediaId,
//     pub size: u64,
//     pub content_type: Mime,
//     pub width: u64,
//     pub height: u64,
// }

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaErrorReason {
    /// this piece of media was not found
    NotFound,

    /// this piece of media was corrupted
    Corrupted,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaQuarantine {
    /// when this media was quarantined
    pub time: Time,

    /// why this media was quarantined
    pub reason: Option<String>,
}

/// An automated scan result
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

    /// A piece of errored media
    Errored {
        /// Why this media is errored
        reason: MediaErrorReason,
    },
}

impl MediaMetadata {
    /// Returns `true` if this media is an image.
    pub fn is_image(&self) -> bool {
        matches!(self, MediaMetadata::Image { .. })
    }

    /// Returns `true` if this media is a video.
    pub fn is_video(&self) -> bool {
        matches!(self, MediaMetadata::Video { .. })
    }

    /// Returns `true` if this media is an audio file.
    pub fn is_audio(&self) -> bool {
        matches!(self, MediaMetadata::Audio { .. })
    }

    /// Returns `true` if this media is a text file.
    pub fn is_text(&self) -> bool {
        matches!(self, MediaMetadata::Text)
    }
}

/// An update to a piece of media
#[derive(Debug, Clone, PartialEq, Eq, lamprey_macros::Diff)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    /// This can only be changed if the media status is not `Consumed`. Once
    /// strip_exif is set to true, cannot be set to false.
    #[cfg_attr(feature = "serde", serde(default))]
    pub strip_exif: Option<bool>,
}

/// a request body for `media_create`
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaCreate {
    /// Whether to strip sensitive exif info, like location or camera make and model.
    ///
    /// Once strip_exif is set to true, cannot be set to false.
    #[cfg_attr(feature = "serde", serde(default))]
    pub strip_exif: bool,

    /// Descriptive alt text, not entirely unlike a caption
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<String>,

    #[cfg_attr(feature = "serde", serde(flatten))]
    #[cfg_attr(feature = "validator", validate(nested))]
    pub source: MediaCreateSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MediaClone {
    /// Set to override the filename
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 256)
    )]
    pub filename: Option<String>,

    /// Descriptive alt text, not entirely unlike a caption
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<String>,
}

/// What to create this media from
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(untagged))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MediaCreated {
    /// The id of the media that has been created
    pub media_id: MediaId,

    /// A url to upload your media to. Is `None` if you used `MediaCreateSource::Download`.
    pub upload_url: Option<Url>,
}

/// describes how this piece of media is linked to another resource
///
/// objects can be linked to multiple objects; for example, media linked to
/// `Message`s also have links to each `MessageVersion` they're referenced in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaLinkType {
    /// this piece of media is linked to a message
    // NOTE: auth checks copy MessageUpdate
    // NOTE: should never exist on its own, always comes with a Message + MessageVersion link
    Message {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    /// this piece of media is linked to a message version
    // NOTE: auth checks copy MessageUpdate
    MessageVersion {
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
    },

    /// this piece of media is used as a user avatar
    // NOTE: auth checks copy UserUpdate
    UserAvatar { user_id: UserId },

    /// this piece of media is used as a user banner
    // NOTE: auth checks copy UserUpdate
    UserBanner { user_id: UserId },

    /// this piece of media is used as a channel icon
    // NOTE: auth checks copy ChannelUpdate
    ChannelIcon { channel_id: ChannelId },

    /// this piece of media is used as a room icon
    // NOTE: auth checks copy RoomUpdate
    RoomIcon { room_id: RoomId },

    /// this piece of media is embedded in a message
    // NOTE: auth checks copy Message
    // NOTE: should never exist on its own, always comes with a Message + MessageVersion link
    Embed { id: EmbedId },

    /// this piece of media is used as a custom emoji
    // NOTE: auth checks copy EmojiUpdate
    CustomEmoji { room_id: RoomId },

    /// this piece of media is used as a room banner
    // NOTE: auth checks copy RoomUpdate
    RoomBanner { room_id: RoomId },

    /// this piece of media is a script
    Script {
        channel_id: ChannelId,
        script_id: RedexId,
    },

    /// this piece of media is a script version
    ScriptVersion {
        channel_id: ChannelId,
        script_id: RedexId,
        version_id: RedexVerId,
    },
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
    #[cfg(feature = "serde")]
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

impl MediaReference {
    pub fn new(media_id: MediaId) -> Self {
        todo!()
    }

    pub fn new_download(url: Url) -> Self {
        todo!()
    }

    pub fn new_attachment(media_index: u64) -> Self {
        todo!()
    }

    pub fn media_id(&self) -> Option<MediaId> {
        match self {
            MediaReference::Media { media_id } => Some(*media_id),
            MediaReference::Url { .. } => None,
            MediaReference::Attachment { .. } => None,
        }
    }
}

impl Media {
    /// create a new errored media
    pub fn errored(id: MediaId, version_id: MediaVerId, reason: MediaErrorReason) -> Self {
        Self {
            id,
            version_id,
            status: MediaStatus::Errored,
            filename: "".to_string(),
            alt: None,
            size: 0,
            content_type: Mime::from_str("application/lamprey-errored-media")
                .expect("always valid"),
            source_url: None,
            metadata: MediaMetadata::Errored { reason },
            user_id: None,
            deleted_at: None,
            quarantine: None,
            scans: vec![],
            has_thumbnail: false,
            has_gifv: false,
            links: vec![],
            room_id: None,
            channel_id: None,
            hashes: Hashes::default(),
            strip_exif: false,
            remote: None,
        }
    }
}
