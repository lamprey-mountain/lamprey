#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    util::{some_option, Diff},
    EmbedCreate, Message, MessageId, MessageType, ParseMentions,
};
use crate::v2::types::media::{Media, MediaReference};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
    #[serde(default)]
    pub attachments: Vec<MessageAttachmentCreate>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 0, max_length = 32)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 0, max = 32)))]
    #[serde(default)]
    pub embeds: Vec<EmbedCreate>,

    #[serde(default)]
    pub mentions: ParseMentions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessagePatch {
    /// the new message content in markdown
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[serde(default, deserialize_with = "some_option")]
    pub content: Option<Option<String>>,

    /// message attachments
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 0, max_length = 32)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 0, max = 32)))]
    pub attachments: Option<Vec<MessageAttachmentCreate>>,

    /// the message this message is replying to
    #[serde(default, deserialize_with = "some_option")]
    pub reply_id: Option<Option<MessageId>>,

    pub embeds: Option<Vec<EmbedCreate>>,
}

/// used in `message_create` and `message_update`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageAttachmentCreate {
    #[serde(flatten)]
    pub media: MediaReference,

    /// Shortcut for setting alt text on the media item
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub alt: Option<Option<String>>,

    /// Shortcut for setting filename on the media item
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 256)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub filename: Option<String>,

    /// if this is a spoiler and should be blurred
    pub spoiler: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageAttachment {
    #[serde(flatten)]
    pub media: Media,

    /// if this is a spoiler and should be blurred
    pub spoiler: bool,
}

impl Diff<Message> for MessagePatch {
    fn changes(&self, other: &Message) -> bool {
        match &other.message_type {
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
