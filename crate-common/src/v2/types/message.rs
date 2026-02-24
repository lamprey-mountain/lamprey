use core::fmt;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::reaction::ReactionCounts;
use crate::v1::types::util::{Diff, Time};
use crate::v1::types::{
    Channel, ChannelId, Mentions, MessageAutomodExecution, MessageCall, MessageChannelIcon,
    MessageChannelMoved, MessageChannelPingback, MessageChannelRename, MessageId, MessageMember,
    MessagePin, MessageThreadCreated, MessageVerId, ParseMentions, Pinned, RoomId, UserId,
};
use crate::v2::types::embed::{Embed, EmbedCreate};
use crate::v2::types::media::{Media, MediaReference};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

pub use crate::v1::types::message::components;

/// a message
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Message {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub room_id: Option<RoomId>,

    // TODO: rename to something better?
    // this is a bit unwieldy, and incorrect if i fetched an old version
    pub latest_version: MessageVersion,

    /// exists if this message is pinned
    pub pinned: Option<Pinned>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub reactions: ReactionCounts,

    /// when this message was deleted
    ///
    /// deleted messages can still be viewed by moderators for a period of time, but otherwise cannot be recovered
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub deleted_at: Option<Time>,

    /// when this message was removed
    ///
    /// removed messages are hidden for non moderators. they are recoverable by moderators
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub removed_at: Option<Time>,

    /// when this message was created
    pub created_at: Time,

    /// the id of who sent this message
    pub author_id: UserId,

    /// the associated thread for this message, if one exists.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub thread: Option<Box<Channel>>,
}

/// a message's content at a point in time
// TODO: add error "latest message version cannot be deleted"
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageVersion {
    pub version_id: MessageVerId,

    /// the id of who this edit. if None, this edit was made by the author
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub author_id: Option<UserId>,

    /// the type and content of this message
    // NOTE: message type generally shouldn't change, but i don't know how to "hoist" the type field to the top level Message struct?
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub message_type: MessageType,

    /// who this message mentioned
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Mentions::is_empty"))]
    pub mentions: Mentions,

    /// when this message version was created, use this as edited_at
    pub created_at: Time,

    /// when this message version was deleted
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub deleted_at: Option<Time>,
}

impl MessageVersion {
    pub fn strip(mut self) -> Self {
        self.message_type = match self.message_type {
            MessageType::DefaultMarkdown(m) => {
                MessageType::DefaultMarkdown(MessageDefaultMarkdown {
                    content: None,
                    attachments: vec![],
                    metadata: None,
                    reply_id: m.reply_id,
                    embeds: vec![],
                })
            }
            m => m,
        };
        self
    }
}

// NOTE: utoipa doesnt seem to like #[deprecated] here
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum MessageType {
    /// a basic message, using markdown
    // NOTE(v2): rename to Default
    DefaultMarkdown(MessageDefaultMarkdown),

    /// a message was pinned
    MessagePinned(MessagePin),

    #[cfg(feature = "feat_message_move")]
    /// (TODO) one or more messages were moved
    MessagesMoved(MessagesMoved),

    /// a thread member was added to the thread or group dm
    MemberAdd(MessageMember),

    /// a thread member was removed from the thread or group dm
    MemberRemove(MessageMember),

    /// a room member joined the room
    MemberJoin,

    /// (TODO) a call was started in a dm or gdm
    Call(MessageCall),

    /// this thread was renamed
    ChannelRename(MessageChannelRename),

    /// (TODO) someone mentioned this thread
    // TODO: rename to ChannelPingback
    // needs some sort of antispam system. again, see github.
    // doesnt necessarily reference a thread in the same room, but usually should
    // maybe don't include in log?
    ChannelPingback(MessageChannelPingback),

    /// this thread was moved
    ChannelMoved(MessageChannelMoved),

    /// The channel's icon was changed
    ChannelIcon(MessageChannelIcon),
    // /// (TODO) receive announcement threads from this room
    // // but where does this get sent to???
    // RoomFollowed(MessageRoomFollowed),
    /// A thread was created from a message
    ThreadCreated(MessageThreadCreated),

    // /// (TODO) interact with a bot, uncertain if i'll go this route
    // BotCommand(MessageBotCommand),
    /// the result of an automod execution
    AutomodExecution(MessageAutomodExecution),
    // /// (TODO) implement a reporting system? uncertain (reports are certain, but reports-as-messages vs as-threads idk)
    // // #[deprecated = "reports will be impl'd as threads"]
    // ModerationReport(MessageModerationReport),

    // /// (TODO) someone nudged you!
    // Nudge,
}

/// a basic message, written using markdown
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageDefaultMarkdown {
    /// the message's content in markdown
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub attachments: Vec<MessageAttachment>,

    /// application defined metadata
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub metadata: Option<MessageMetadata>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub embeds: Vec<Embed>,
}

/// arbitrary key-value metadata included for a message.
///
/// - max 8 keys
/// - max 32 chars per key
/// - max 1024 chars per value
/// - max 2048 chars across all values
///
/// included in interaction. only visible to user who sent it (and the owner if its a bot).
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageMetadata(pub HashMap<String, String>);

impl MessageMetadata {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn insert(&mut self, key: String, value: String) -> Option<String> {
        self.0.insert(key, value)
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }
}

impl FromIterator<(String, String)> for MessageMetadata {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(feature = "validator")]
mod v {
    use serde_json::json;
    use validator::{Validate, ValidateLength, ValidationError, ValidationErrors};

    use super::MessageMetadata;

    impl Validate for MessageMetadata {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();
            let mut total_value_len = 0;

            // validate number of keys (max 8)
            if !self.0.validate_length(None, Some(8), None) {
                let mut err = ValidationError::new("length");
                err.add_param("max".into(), &json!(8));
                err.add_param("actual".into(), &(self.0.len() as i64));
                errors.add("data", err);
            }

            for (key, value) in self.0.iter() {
                // validate key length
                if !key.validate_length(Some(1), Some(32), None) {
                    let mut err = ValidationError::new("key_length");
                    err.add_param("max".into(), &json!(32));
                    err.add_param("min".into(), &json!(1));
                    err.add_param("actual".into(), &(key.len() as i64));
                    errors.add("key", err);
                }

                // validate value length
                if !value.validate_length(None, Some(1024), None) {
                    let mut err = ValidationError::new("value_length");
                    err.add_param("max".into(), &json!(1024));
                    err.add_param("actual".into(), &(value.len() as i64));
                    errors.add("value", err);
                }

                total_value_len += value.len();
            }

            // validate total value length
            if total_value_len > 2048 {
                let mut err = ValidationError::new("total_value_length");
                err.add_param("max".into(), &json!(2048));
                err.add_param("actual".into(), &(total_value_len as i64));
                errors.add("total_value", err);
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }
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

    /// application defined metadata
    pub metadata: Option<MessageMetadata>,
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

    /// application defined metadata
    ///
    /// passing this will replace metadata
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub metadata: Option<Option<MessageMetadata>>,
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

    #[cfg(feature = "feat_message_forwarding")]
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

    #[cfg(feature = "feat_message_forwarding")]
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

impl MessageType {
    // TODO: return if this is deletable by sender, not deletable by sender, or not deletable at all (even by mods)
    pub fn is_deletable(&self) -> bool {
        match self {
            MessageType::DefaultMarkdown(_) => true,
            #[cfg(feature = "feat_message_forwarding")]
            MessageType::Forward(_) => true,
            MessageType::MessagePinned(_) => true,
            MessageType::MemberAdd(_) => false,
            MessageType::MemberRemove(_) => false,
            MessageType::MemberJoin => true,
            MessageType::ChannelRename(_) => false,
            MessageType::ChannelPingback(_) => true,
            MessageType::ChannelIcon(_) => false,
            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(_) => false,
            MessageType::Call(_) => false,
            MessageType::ThreadCreated(_) => false,
            MessageType::ChannelMoved(_) => false,

            // NOTE: this should require the MessageDelete permission
            MessageType::AutomodExecution(_) => true,
        }
    }

    pub fn is_editable(&self) -> bool {
        matches!(self, MessageType::DefaultMarkdown(_))
    }

    /// if threads can be created from this message
    pub fn is_threadable(&self) -> bool {
        matches!(self, MessageType::DefaultMarkdown(_))
    }

    pub fn is_movable(&self) -> bool {
        matches!(self, MessageType::DefaultMarkdown(_))
    }

    /// if this will be returned in the thread activity route
    pub fn is_activity(&self) -> bool {
        match self {
            MessageType::DefaultMarkdown(_) => false,
            #[cfg(feature = "feat_message_forwarding")]
            MessageType::Forward(_) => false,
            MessageType::MessagePinned(_) => true,
            MessageType::MemberAdd(_) => true,
            MessageType::MemberRemove(_) => true,
            MessageType::MemberJoin => false,
            MessageType::ChannelRename(_) => true,
            MessageType::ChannelPingback(_) => true,
            MessageType::ChannelIcon(_) => true,
            MessageType::ChannelMoved(_) => true,
            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(_) => false,
            MessageType::Call(_) => false,
            MessageType::ThreadCreated(_) => true,
            MessageType::AutomodExecution(_) => false,
        }
    }
}

impl MessageCreate {
    pub fn is_empty(&self) -> bool {
        self.content.as_ref().is_none_or(|s| s.is_empty())
            && self.attachments.is_empty()
            && self.embeds.is_empty()
    }
}

impl MessageDefaultMarkdown {
    pub fn is_empty(&self) -> bool {
        self.content.as_ref().is_none_or(|s| s.is_empty())
            && self.attachments.is_empty()
            && self.embeds.is_empty()
    }
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: i probably want a better Display impl than using fmt_tantivy
        self.fmt_tantivy(f)
    }
}

impl MessageType {
    /// format a message for tantivy search indexing
    pub fn fmt_tantivy(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: include ids (eg. user id in MemberAdd, or message id in MessagePinned)
        match self {
            MessageType::DefaultMarkdown(m) => {
                if let Some(content) = &m.content {
                    write!(f, "{}", content)
                } else {
                    write!(f, "")
                }
            }
            MessageType::MessagePinned(_) => {
                write!(f, "message pinned")
            }
            MessageType::MemberAdd(_) => {
                write!(f, "member add")
            }
            MessageType::MemberRemove(_) => {
                write!(f, "member removed")
            }
            MessageType::MemberJoin => {
                write!(f, "member joined")
            }
            MessageType::Call(call_msg) => {
                if call_msg.ended_at.is_some() {
                    write!(f, "call (ended)")
                } else {
                    write!(f, "call (active)")
                }
            }
            MessageType::ChannelRename(rename) => {
                write!(f, "channel renamed from to \"{}\"", rename.name_new)
            }
            MessageType::ChannelPingback(_) => {
                write!(f, "channel pingback")
            }
            MessageType::ChannelMoved(_) => {
                write!(f, "channel moved")
            }
            MessageType::ChannelIcon(_) => {
                write!(f, "channel icon changed")
            }
            MessageType::ThreadCreated(thread_msg) => {
                if thread_msg.source_message_id.is_some() {
                    write!(f, "thread created from message")
                } else {
                    write!(f, "thread created")
                }
            }
            MessageType::AutomodExecution(_exec) => {
                write!(f, "auto moderation action executed")

                // TODO: log rule names, matches, etc
                // write!(
                //     f,
                //     "Auto moderation action executed",
                //     automod_msg.matches[0].matches,
                //     automod_msg.rules.iter().map(|r| r.name.as_str()).join(" ")
                // )
            }
            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(move_msg) => {
                write!(f, "messages moved ",)
            }
        }
    }
}
