#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    misc::Time,
    reaction::ReactionCounts,
    util::{some_option, Diff},
    Channel, ChannelId, Embed, EmbedCreate, Mentions, MessageDefaultMarkdown, MessageId,
    MessageType, MessageVerId, ParseMentions, Pinned, UserId,
};
use crate::v2::types::media::{Media, MediaReference};

/// a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Message {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub latest_version: MessageVersion,

    /// exists if this message is pinned
    pub pinned: Option<Pinned>,

    #[serde(default)]
    pub reactions: ReactionCounts,

    /// when this message was deleted
    ///
    /// deleted messages can still be viewed by moderators for a period of time, but otherwise cannot be recovered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<Time>,

    /// when this message was removed
    ///
    /// removed messages are hidden for non moderators. they are recoverable by moderators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed_at: Option<Time>,

    /// when this message was created
    pub created_at: Time,

    /// the id of who sent this message
    pub author_id: UserId,

    /// the associated thread for this message, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread: Option<Box<Channel>>,
}

/// a message at a point in time
// TODO: add error "latest message version cannot be deleted"
// TODO: strip content instead of deleting
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageVersion {
    pub version_id: MessageVerId,

    /// the id of who this edit. if None, this edit was made by the author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_id: Option<UserId>,

    /// the type and content of this message
    #[serde(flatten)]
    pub message_type: MessageType,

    pub mentions: Mentions,

    /// when this message version was created, use this as edited_at
    pub created_at: Time,

    /// when this message version was deleted
    pub deleted_at: Option<Time>,
}

impl MessageVersion {
    pub fn strip(mut self) -> Self {
        self.mentions = Mentions::default();
        self.message_type = match self.message_type {
            MessageType::DefaultMarkdown(m) => {
                MessageType::DefaultMarkdown(MessageDefaultMarkdown {
                    content: None,
                    attachments: vec![],
                    metadata: None,
                    reply_id: m.reply_id,
                    embeds: vec![],
                    override_name: None,
                })
            }
            m => m,
        };
        self
    }
}

/// a basic message
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    // #[serde(flatten)]
    // pub inner: AttachmentType,
    /// if this is a spoiler and should be blurred
    pub spoiler: bool,
}

/// for forwards
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageSnapshot {
    #[serde(flatten)]
    pub message_type: MessageType,
}

// // forwards as attachments?
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum AttachmentType {
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
