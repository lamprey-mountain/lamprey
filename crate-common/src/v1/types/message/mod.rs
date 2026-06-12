#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::{Validate, ValidateLength, ValidationError, ValidationErrors};

use crate::v1::types::automod::{AutomodAction, AutomodMatches, AutomodRuleStripped};
use crate::v1::types::components::{self, Components};
use crate::v1::types::e2ee::media::EncryptedMedia;
use crate::v1::types::e2ee::MlsEpoch;
use crate::v1::types::flume::MessageFlume;
use crate::v1::types::metadata::Metadata;
use crate::v1::types::misc::binary::Binary;
use crate::v1::types::moderation::Report;
use crate::v1::types::reaction::ReactionCounts;
use crate::v1::types::util::{Diff, Time};
use crate::v1::types::{ApplicationId, AuditLogEntry, Embed, InteractionId, RoleId, UserId};
use crate::v1::types::{ChannelType, EmojiId, MediaId, RoomId};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use crate::v2::types::media::{Media, MediaReference};

use super::channel::Channel;
use super::EmbedCreate;
use super::{ChannelId, MessageId, MessageVerId};
use std::fmt;

pub mod flume;
pub mod metadata;

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

    /// the associated flume for this message, if one exists.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub flume: Option<MessageFlume>,

    /// the associated interaction for this message, if one exists.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub interaction: Option<MessageInteraction>,

    /// whether this message is ephemeral
    ///
    /// ephemeral messages are only visible to the user who created an interaction and aren't stored
    #[cfg_attr(feature = "serde", serde(default))]
    pub ephemeral: bool,
}

impl Message {
    pub fn reply_id(&self) -> Option<MessageId> {
        self.latest_version.reply_id
    }
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

    /// the message this version is replying to
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reply_id: Option<MessageId>,

    /// the type and content of this message
    // NOTE: message type generally shouldn't change, but i don't know how to "hoist" the type field to the top level Message struct?
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub message_type: MessageType,

    /// who this message mentioned
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Mentions::is_empty")
    )]
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
                    components: Components::default(),
                })
            }
            m => m,
        };
        self
    }
}

/// information about a pinned message
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Pinned {
    /// when this was pinned
    pub time: Time,

    /// the position of this pin. lower numbers come first.
    pub position: u16,
}

/// reorder pinned messages
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct PinsReorder {
    /// the messages to reorder
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub messages: Vec<PinsReorderItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct PinsReorderItem {
    pub id: MessageId,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub position: Option<Option<u16>>,
}

fn true_fn() -> bool {
    true
}

/// what mentions to parse from the message content. mentions will only be parsed if the message content actually contains a mention pattern.
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ParseMentions {
    /// only parse mentions for these users. an empty vec disables all mentions, while None allows all mentions.
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 128))]
    pub users: Option<Vec<UserId>>,

    /// only parse mentions for these roles. an empty vec disables all mentions, while None allows all mentions.
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 128))]
    pub roles: Option<Vec<RoleId>>,

    /// whether to parse @everyone mentions from the content
    #[cfg_attr(feature = "serde", serde(default = "true_fn"))]
    pub everyone: bool,
}

/// who/what this message notified on send
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Mentions {
    /// the users that were mentioned
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 128))]
    pub users: Vec<MentionsUser>,

    /// the roles that were mentioned
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 128))]
    pub roles: Vec<MentionsRole>,

    /// the channels that were mentioned
    // NOTE: this may not be necessary; the user should already have all channels. this is only needed for forwards, but in that case do i really want to leak channel names/types?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 128))]
    pub channels: Vec<MentionsChannel>,

    /// the custom emojis that were used in this message
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 128))]
    pub emojis: Vec<MentionsEmoji>,

    /// if this message mentions everyone
    #[cfg_attr(feature = "serde", serde(default))]
    pub everyone: bool,
}

impl Mentions {
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
            && self.roles.is_empty()
            && self.channels.is_empty()
            && self.emojis.is_empty()
            && !self.everyone
    }
}

/// a mentioned user
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MentionsUser {
    /// the id of this user
    pub id: UserId,

    /// the resolved name (either the room member nickname or the user's name)
    pub resolved_name: String,
}

/// a mentioned role
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MentionsRole {
    /// the id of this role
    pub id: RoleId,
    // // TODO: add this
    // /// the name of this role
    // pub name: String,
}

/// a mentioned channel
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MentionsChannel {
    /// the id of this channel
    pub id: ChannelId,

    /// the room this is in
    pub room_id: Option<RoomId>,

    /// the type of this channel
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ChannelType,

    /// the name of this channel
    pub name: String,
}

/// a custom emoji used in the message
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MentionsEmoji {
    /// the id of this emoji
    pub id: EmojiId,

    /// the name of this emoji
    pub name: String,

    /// if this emoji is animated
    pub animated: bool,
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
    pub metadata: Option<Metadata>,

    /// the components for this message
    pub components: Option<Components<components::Create>>,

    /// whether to make this message ephemeral
    pub ephemeral: bool,
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
    pub metadata: Option<Option<Metadata>>,

    /// the components for this message
    pub components: Option<Components<components::Create>>,
}

// NOTE: utoipa doesnt seem to like #[deprecated] here
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum MessageType {
    /// a basic message, using markdown
    DefaultMarkdown(MessageDefaultMarkdown),

    /// an encrypted message
    #[cfg(feature = "feat_e2ee")]
    Encrypted(MessageEncrypted),

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

// impl fmt::Display for MessageType {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             MessageType::DefaultMarkdown(_) => write!(f, "DefaultMarkdown"),
//             #[cfg(feature = "feat_e2ee")]
//             MessageType::Encrypted(_) => write!(f, "Encrypted"),
//             MessageType::MessagePinned(_) => write!(f, "MessagePinned"),
//             #[cfg(feature = "feat_message_move")]
//             MessageType::MessagesMoved(_) => write!(f, "MessagesMoved"),
//             MessageType::MemberAdd(_) => write!(f, "MemberAdd"),
//             MessageType::MemberRemove(_) => write!(f, "MemberRemove"),
//             MessageType::MemberJoin => write!(f, "MemberJoin"),
//             MessageType::Call(_) => write!(f, "Call"),
//             MessageType::ChannelRename(_) => write!(f, "ChannelRename"),
//             MessageType::ChannelPingback(_) => write!(f, "ChannelPingback"),
//             MessageType::ChannelMoved(_) => write!(f, "ChannelMoved"),
//             MessageType::ChannelIcon(_) => write!(f, "ChannelIcon"),
//             MessageType::ThreadCreated(_) => write!(f, "ThreadCreated"),
//             MessageType::AutomodExecution(_) => write!(f, "AutomodExecution"),
//         }
//     }
// }

/// Information about a message being pinned
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessagePin {
    pub pinned_message_id: MessageId,
}

/// Information about an auto moderation execution
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageAutomodExecution {
    /// the rules that were triggered
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 32))]
    pub rules: Vec<AutomodRuleStripped>,

    /// the actions that were executed
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 32))]
    pub actions: Vec<AutomodAction>,

    /// the content that was matched
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 32))]
    pub matches: Vec<AutomodMatches>,

    /// the user who triggered this execution
    pub user_id: UserId,

    /// the id of the channel where the message was sent, is None if this is not a message
    // NOTE: this is only populated if the target is a mesage
    // TODO: design thread and other automod target types
    pub channel_id: Option<ChannelId>,

    /// if the message wasn't blocked, this is the id of it
    pub flagged_message_id: Option<MessageId>,
    // pub completed: Option<AutomodAlertCompleted>,
}

// struct AutomodAlertCompleted {
//     /// the user who completed this alert
//     user_id: UserId,

//     /// when this alert was completed at
//     completed_at: Time,
// }

/// Information about a thread being renamed
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageChannelRename {
    #[cfg_attr(feature = "serde", serde(alias = "new"))]
    pub name_new: String,

    #[cfg_attr(feature = "serde", serde(alias = "old"))]
    pub name_old: String,
}

/// Information about a thread being moved
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageChannelMoved {
    pub parent_id_old: Option<ChannelId>,
    pub parent_id_new: Option<ChannelId>,
}

/// Information about a thread being created
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageThreadCreated {
    /// the message this thread was created from
    pub source_message_id: Option<MessageId>,

    /// the id of the thread that was created
    // FIXME: this shouldn't be an Option
    pub thread_id: Option<ChannelId>,
}

/// Information about the pingback
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageChannelPingback {
    pub source_room_id: RoomId,
    pub source_channel_id: ChannelId,
    pub source_user_id: UserId,
}

/// Information about a channel icon change
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageChannelIcon {
    pub icon_id_old: Option<MediaId>,
    pub icon_id_new: Option<MediaId>,
}

#[cfg(feature = "feat_message_move")]
/// Information about one or more messages being moved between threads
/// probably want this being sent in both the source and target threads, maybe
/// with a bit of different styling depending on whether its source/target
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessagesMoved {
    // do messages keep their ids when being moved?
    pub start_id: MessageId,
    pub end_id: MessageId,
    pub source_id: ChannelId,
    pub target_id: ChannelId,
    pub reason: Option<String>,
}

/// Information about a member being added or removed from a thread
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageMember {
    pub target_user_id: UserId,
}

/// Following a room and will receive announcement posts from it
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageRoomFollowed {
    pub thread_id: ChannelId,
    pub reason: Option<String>,
}

// TODO: remove
/// audit log entries as a message (builtin moderation logging?)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageModerationLog {
    pub audit_log_entry: AuditLogEntry,
}

/// a report that moderators should look at
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageModerationReport {
    pub report: Report,
}

/// a bot command
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageBotCommand {
    pub command_id: String,
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
    pub metadata: Option<Metadata>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub embeds: Vec<Embed>,

    /// the components for this message
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Components::is_empty")
    )]
    pub components: Components<components::Canonical>,
}

/// an encrypted message
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageEncrypted {
    // TODO: find an appropriate size limit for this (how much overhead does mls cause?)
    /// encrypted content of the message
    ///
    /// - decrypts into a MessageDefaultMarkdownEncrypted struct
    /// - encrypted with aes-256-gcm
    pub ciphertext: Binary<65536>,

    // TODO: pub alg: EncryptionAlgorithm,
    // TODO: find an appropriate size limit for this (how much overhead does mls cause?)
    /// the nonce for the ciphertext
    pub nonce: Binary<256>,

    /// the media this message is attached to, for garbage collection
    pub media: Vec<Media>,

    pub epoch: MlsEpoch,
}

/// a basic message, written using markdown. for use with e2ee.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageDefaultMarkdownEncrypted {
    /// the message's content in markdown
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub attachments: Vec<MessageAttachmentEncrypted>,

    /// application defined metadata
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub metadata: Option<Metadata>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub embeds: Vec<Embed>,

    /// the components for this message
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Components::is_empty")
    )]
    pub components: Components<components::Encrypted>,
}

/// used in `message_create` and `message_update`
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageAttachmentEncrypted {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: MessageAttachmentEncryptedType,

    /// if this is a spoiler and should be blurred
    pub spoiler: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MessageAttachmentEncryptedType {
    /// a piece of media
    Media { media: EncryptedMedia },

    #[cfg(feature = "feat_message_forwarding")]
    /// a forwarded message
    Forward { snapshot: MessageSnapshot },
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
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Mentions::is_empty")
    )]
    pub mentions: Mentions,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MessageAttachmentCreateType {
    Media {
        #[cfg_attr(feature = "serde", serde(flatten))]
        media: MediaReference,

        /// Shortcut for setting alt text on the media item
        #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
        alt: Option<Option<String>>,

        /// Shortcut for setting filename on the media item
        #[cfg_attr(
            feature = "utoipa",
            schema(required = false, min_length = 1, max_length = 256)
        )]
        filename: Option<String>,
    },

    #[cfg(feature = "feat_message_forwarding")]
    Forward {
        channel_id: ChannelId,
        message_id: MessageId,
    },
}

#[cfg(feature = "validator")]
impl Validate for MessageAttachmentCreateType {
    fn validate(&self) -> Result<(), ValidationErrors> {
        use serde_json::json;

        let mut errors = ValidationErrors::new();

        match self {
            MessageAttachmentCreateType::Media { alt, filename, .. } => {
                if let Some(Some(alt_val)) = alt {
                    if !alt_val.validate_length(None, Some(8192), None) {
                        let mut err = ValidationError::new("length");
                        err.add_param("max".into(), &json!(8192));
                        err.add_param("actual".into(), &(alt_val.len() as i64));
                        errors.add("alt", err);
                    }
                }

                if let Some(filename_val) = filename {
                    if !filename_val.validate_length(Some(1), Some(256), None) {
                        let mut err = ValidationError::new("length");
                        err.add_param("min".into(), &json!(1));
                        err.add_param("max".into(), &json!(256));
                        err.add_param("actual".into(), &(filename_val.len() as i64));
                        errors.add("filename", err);
                    }
                }
            }
            #[cfg(feature = "feat_message_forwarding")]
            MessageAttachmentCreateType::Forward { .. } => {}
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageCall {
    /// when the call ended. is None if the call is still going.
    pub ended_at: Option<Time>,

    /// the people who joined the call
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 128))]
    pub participants: Vec<UserId>,
}

/// the interaction that caused this message to be sent
// who is the message author?
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageInteraction {
    pub id: InteractionId,
    pub application_id: ApplicationId,

    /// the user who triggered this interaction
    pub user_id: UserId,

    /// the interaction's source message
    ///
    /// if this interaction was triggered by a message component (eg. a button), this is the id of the message the component was on
    pub source_message_id: Option<MessageId>,
}

/// the current status
#[cfg(feature = "feat_interaction_reaction")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum InteractionStatus {
    /// This message is still loading, or the action it represents is in progress
    ///
    /// - Will switch to Failed after 5 minutes or 30 seconds without edit
    /// - Can edit without creating message history entry
    /// - Intended for dynamic/streaming responses
    Loading,

    /// The (inter)action this message represents failed
    Failed {
        reason: String,
        // code: InteractionStatusKnownErrorCode,
        can_retry: bool,
    },
}

// enum InteractionStatusKnownErrorCode {
//     Forbidden,
//     Timeout,
//     BadInput,
//     Missing,
//     Conflict,
//     Gone,
//     TooLarge,
//     Cancelled,
//     Ratelimit,
// }

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageMove {
    /// which messages to move
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 128)))]
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 128))]
    pub message_ids: Vec<MessageId>,

    /// the channel to move the messages to
    ///
    /// must be in same room (for now...)
    pub target_channel_id: ChannelId,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageModerate {
    /// which messages to delete
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 128))]
    pub delete: Vec<MessageId>,

    /// which messages to remove
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 128))]
    pub remove: Vec<MessageId>,

    /// which messages to restore
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    #[cfg_attr(feature = "utoipa", schema(min_length = 0, max_length = 128))]
    pub restore: Vec<MessageId>,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RepliesQuery {
    /// how deeply to fetch replies
    #[cfg_attr(feature = "serde", serde(default = "fn_one"))]
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 8)))]
    pub depth: u16,

    /// how many replies to fetch per branch
    pub breadth: Option<u16>,

    /// which parent message to fetch replies from, where 0 is the message itself, 1 is its parent, and so on.
    pub context: Option<u16>,
}

/// a response to a replies query
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RepliesResponse {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub children: RepliesChildren,
}

/// a single message for a replies query
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RepliesMessage {
    /// the message itself
    pub message: Message,

    /// the children for this message
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub children: RepliesChildren,
}

/// a list of children for a RepliesItem or the top level
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RepliesChildren {
    /// the children for this message
    pub children: Vec<RepliesMessage>,

    /// the total number of replies to this message
    pub count_direct: u64,

    /// the total number of replies to this message, calculated recursively
    pub count_recursive: u64,

    /// the current depth of this message in the tree, or 0 for the top level
    pub depth: u64,

    /// cursor that can be used to fetch more
    pub cursor: Option<String>,

    /// whether there are more messages after the end of the children array
    pub has_more: bool,
}

/// always returns one
fn fn_one() -> u16 {
    1
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct ContextQuery {
    pub to_start: Option<MessageId>,
    pub to_end: Option<MessageId>,
    pub limit: Option<u16>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RatelimitPut {
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub slowmode_thread_expire_at: Option<Option<Time>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub slowmode_message_expire_at: Option<Option<Time>>,
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ContextResponse {
    pub items: Vec<Message>,
    pub total: u64,
    pub has_after: bool,
    pub has_before: bool,
}

impl Diff for MessagePatch {
    type Target = Message;

    fn changes(&self, other: &Message) -> bool {
        match &other.latest_version.message_type {
            MessageType::DefaultMarkdown(m) => {
                // content: Option<Option<String>> vs Option<String>
                if let Some(ref val) = self.content {
                    if val != &m.content {
                        return true;
                    }
                }
                // reply_id: Option<Option<MessageId>> vs Option<MessageId>
                if let Some(ref val) = self.reply_id {
                    if val != &m.reply_id {
                        return true;
                    }
                }
                if self.embeds.is_some() {
                    return true;
                }
                // metadata: Option<Option<MessageMetadata>> vs Option<MessageMetadata>
                if let Some(ref val) = self.metadata {
                    if val != &m.metadata {
                        return true;
                    }
                }
                if self.attachments.as_ref().is_some_and(|a| {
                    a.len() != m.attachments.len()
                        || a.iter().zip(&m.attachments).any(|(a, b)| {
                            if a.spoiler != b.spoiler {
                                return true;
                            }

                            match (&a.ty, &b.ty) {
                                (
                                    MessageAttachmentCreateType::Media {
                                        media,
                                        alt,
                                        filename,
                                    },
                                    MessageAttachmentType::Media {
                                        media: existing_media,
                                    },
                                ) => {
                                    match media {
                                        MediaReference::Media { media_id } => {
                                            if *media_id != existing_media.id {
                                                return true;
                                            }
                                        }
                                        // if we're not referencing the media by id, we're uploading/downloading it
                                        _ => return true,
                                    }

                                    // alt: Option<Option<String>> vs existing_media.alt: Option<String>
                                    (if let Some(alt_val) = alt {
                                        alt_val != &existing_media.alt
                                    } else {
                                        false
                                    }) || filename
                                        .as_ref()
                                        .is_some_and(|f| f != &existing_media.filename)
                                }
                                #[cfg(feature = "feat_message_forwarding")]
                                (
                                    MessageAttachmentCreateType::Forward {
                                        channel_id,
                                        message_id,
                                    },
                                    MessageAttachmentType::Forward { snapshot },
                                ) => {
                                    *channel_id != snapshot.channel_id
                                        || *message_id != snapshot.message_id
                                }
                                #[allow(unreachable_patterns)]
                                _ => true,
                            }
                        })
                }) {
                    return true;
                }
                false
            }
            // this edit is invalid!
            _ => false,
        }
    }

    fn apply(self, mut other: Self::Target) -> Self::Target {
        if let MessageType::DefaultMarkdown(ref mut m) = other.latest_version.message_type {
            if let Some(val) = self.content {
                m.content = val;
            }
            if let Some(val) = self.reply_id {
                m.reply_id = val;
            }
            // TODO: handle embeds apply
            if let Some(val) = self.metadata {
                m.metadata = val;
            }
            // Note: attachments apply requires From<MessageAttachmentCreate> for MessageAttachment
        }
        other
    }
}

impl MessageType {
    // TODO: return if this is deletable by sender, not deletable by sender, or not deletable at all (even by mods)
    pub fn is_deletable(&self) -> bool {
        match self {
            MessageType::DefaultMarkdown(_) => true,
            #[cfg(feature = "feat_e2ee")]
            MessageType::Encrypted(_) => true,
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
    // NOTE: update these queries when is_activity is updated
    // crate-backend-data-postgres/sql/message_activity_paginate.sql
    // crate-backend-data-postgres/sql/message_activity_count.sql
    pub fn is_activity(&self) -> bool {
        match self {
            MessageType::DefaultMarkdown(_) => false,
            #[cfg(feature = "feat_e2ee")]
            MessageType::Encrypted(_) => false,
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

    /// remove all content from this message
    pub fn strip(&mut self) {
        self.content = None;
        self.attachments = vec![];
        self.embeds = vec![];
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
            #[cfg(feature = "feat_e2ee")]
            MessageType::Encrypted(e) => {
                write!(f, "encrypted ({} byte ciphertext)", e.ciphertext.len())
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
