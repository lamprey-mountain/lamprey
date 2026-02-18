#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::util::{Diff, Time};
use crate::v1::types::{
    ChannelId, Embed, EmbedCreate, Mentions, MessageId, MessageType, MessageVerId, ParseMentions,
    RoomId,
};
use crate::v2::types::media::{Media, MediaReference};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

pub use crate::v1::types::{Message, MessageVersion};

/// a basic message
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageContent {
    /// the message's content in markdown
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    // TODO(#325): use MediaRef here during create
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub attachments: Vec<Media>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub embeds: Vec<Embed>,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageCreate {
    /// the message's content in markdown
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    /// message attachments
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 0, max_length = 32)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 0, max = 32)))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub attachments: Vec<MessageAttachmentCreate>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 0, max_length = 32)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 0, max = 32)))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub embeds: Vec<EmbedCreate>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub mentions: ParseMentions,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessagePatch {
    /// the new message content in markdown
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub content: Option<Option<String>>,

    /// message attachments
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 0, max_length = 32)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 0, max = 32)))]
    pub attachments: Option<Vec<MessageAttachmentCreate>>,

    /// the message this message is replying to
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub reply_id: Option<Option<MessageId>>,

    pub embeds: Option<Vec<EmbedCreate>>,
}

/// used in `message_create` and `message_update`
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageAttachmentCreate {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: MessageAttachmentCreateType,

    /// if this is a spoiler and should be blurred
    #[cfg_attr(feature = "serde", serde(default))]
    pub spoiler: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageAttachment {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: MessageAttachmentType,

    /// if this is a spoiler and should be blurred
    pub spoiler: bool,
}

/// a snapshot of a message at a point in time, for forwards
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageSnapshot {
    pub room_id: Option<RoomId>,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub version_id: MessageVerId,
    pub created_at: Time,
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub message_type: MessageType,

    /// who this message mentioned
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Mentions::is_empty"))]
    pub mentions: Mentions,
}

// FIXME: validator
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum MessageAttachmentCreateType {
    Media {
        #[cfg_attr(feature = "serde", serde(flatten))]
        media: MediaReference,

        /// Shortcut for setting alt text on the media item
        #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
        // #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
        alt: Option<Option<String>>,

        /// Shortcut for setting filename on the media item
        #[cfg_attr(
            feature = "utoipa",
            schema(required = false, min_length = 1, max_length = 256)
        )]
        // #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
        filename: Option<String>,
    },

    Forward {
        channel_id: ChannelId,
        message_id: MessageId,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum MessageAttachmentType {
    /// a piece of media
    // or should this be called File? should i differentiate files and media?
    Media { media: Media },

    /// a forwarded message
    Forward { snapshot: MessageSnapshot },
    // should i have Embed for explicitly added embeds vs generated embeds?
    // TODO: Geolocation,
    // TODO: Moderation, (automod execution? or should this be a message type?)
}

impl Diff<Message> for MessagePatch {
    fn changes(&self, other: &Message) -> bool {
        match &other.latest_version.message_type {
            MessageType::DefaultMarkdown(m) => {
                self.content.changes(&m.content)
                    || self.reply_id.changes(&m.reply_id)
                    || self.embeds.is_some()
                // FIXME: diff checking for `MessageAttachment`s
                // || self.attachments.as_ref().is_some_and(|a| {
                //     a.len() != m.attachments.len()
                //         || a.iter().zip(&m.attachments).any(|(a, b)| {
                //             let is_same_media = match &a.media {
                //                 MediaReference::Media { media_id } => media_id == b.id,
                //                 _ => return true,
                //             };
                //             a.alt.changes(b.alt)
                //                 || a.filename.changes(b.filename)
                //                 || a.spoiler.changes(b.)
                //         })
                // })
            }
            // this edit is invalid!
            _ => false,
        }
    }
}
