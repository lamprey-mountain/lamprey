//! media schema v3

use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{text::Language, EmbedId, MediaId, MessageId, MessageVerId, Mime, UserId};

pub mod embed;
pub mod file;
pub mod stream;

pub use embed::Embed;
pub use file::*;
pub use stream::Streamable;

/// a piece of media. becomes immutable after being linked to something.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Media<T: MediaType> {
    /// an unique identifier
    pub id: MediaId,

    /// if the associated object has been deleted, flag for garbage collection
    pub is_deleted: bool,

    /// if the nsfw scanner detected something
    pub is_likely_nsfw: bool,

    /// cannot be accessed by regular users
    pub is_quarantined: bool,

    /// what this media is linked to. each piece of media may only be linked to one thing. if None, this media hasn't been consumed yet.
    pub link: Option<MediaLink>,

    /// who created this piece of media
    // (should i really expose this publically, or make it an Option?)
    pub user_id: UserId,

    /// info/metadata about this media
    pub info: T,

    /// Descriptive alt text, not entirely unlike a caption.
    /// Used by screenreaders and as a fallback if this media fails to load
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<String>,
}

/// what object a piece of media is linked to
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MediaLink {
    /// linked to a message
    Message {
        /// the id of the message this is linked to
        message_id: MessageId,

        /// the earliest version_id this media is linked to
        version_id: MessageVerId,
    },

    /// linked to a user's avatar
    AvatarUser {
        /// the id of the user this is linked to
        user_id: UserId,
    },

    /// linked to a url or custom embed
    Embed {
        /// the id of the embed this is linked to
        embed_id: EmbedId,
    },
}

pub trait MediaType {
    fn tag(&self) -> &'static str;
}

impl<T: MediaType> MediaType for File<T> {
    fn tag(&self) -> &'static str {
        self.meta.tag()
    }
}

macro_rules! impl_media_type {
    ($name:ident) => {
        paste::paste! {
            pub type [<Media $name>] = Media<$name>;
        }

        impl MediaType for $name {
            fn tag(&self) -> &'static str {
                stringify!($name)
            }
        }
    };
}

impl_media_type!(Image);
impl_media_type!(Video);
impl_media_type!(Audio);
impl_media_type!(Streamable);
impl_media_type!(Text);
impl_media_type!(Generic);
impl_media_type!(Embed);

/// Any file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MediaFile {
    Image(Media<FileImage>),
    Video(Media<FileVideo>),
    Audio(Media<FileAudio>),
    Text(Media<FileText>),
    File(Media<FileGeneric>),
    // do i want to include this?
    // Streamable(Media<Streamable>),
}

/// Any piece of media whatsoever
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MediaAny {
    Image(Media<FileImage>),
    Video(Media<FileVideo>),
    Audio(Media<FileAudio>),
    Text(Media<FileText>),
    File(Media<FileGeneric>),
    Streamable(Media<Streamable>),
    Embed(Media<Embed>),
}

/// a message attachment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub struct Attachment {
    pub media: MediaAny,

    /// if this piece of media is a spoiler in context
    pub is_spoiler: bool,

    /// override alt text with possibly more contextual text
    // somewhat unnecessary with UrlEmbed? could/should i enforce this with
    // types, somehow?
    pub alt_override: Option<String>,
}

// i need more tests
#[cfg(test)]
mod tests {
    use super::MediaAny;

    #[test]
    fn test_roundtrip() {
        let val = serde_json::json!({
            "type": "Image",
            "id": "6f8bc7a5-a628-4a01-9bea-48b57c3b1036",
            "user_id": "555509b9-edba-4a8a-a51b-f367501a4f5f",
            "alt": "a test image",
            "is_deleted": false,
            "is_likely_nsfw": false,
            "is_quarantined": false,
            "link": null,
            "info": {
                "filename": "test.png",
                "size": 1234,
                "mime": "image/png",
                "source_url": null,
                "height": 123,
                "width": 456,
            }
        });
        let parsed: MediaAny = serde_json::from_value(val.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), val);
    }
}
