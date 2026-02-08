#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::automod::{AutomodAction, AutomodMatches, AutomodRuleStripped};
use crate::v1::types::moderation::Report;
use crate::v1::types::reaction::ReactionCounts;
#[cfg(feature = "feat_interaction_reaction")]
use crate::v1::types::reaction::ReactionKey;
use crate::v1::types::util::{some_option, Diff, Time};
use crate::v1::types::{AuditLogEntry, Embed, RoleId, UserId};
use crate::v1::types::{ChannelType, EmojiId, MediaId, RoomId};

use crate::v2::types::message::Message as MessageV2;

use super::channel::Channel;
use super::EmbedCreate;
use super::{
    media::{Media, MediaRef},
    ChannelId, MessageId, MessageVerId,
};

pub mod components;

/// a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Message {
    pub id: MessageId,
    pub channel_id: ChannelId,

    // TODO: rename to something better?
    // this is a bit unwieldy, and incorrect if i fetched an old version
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

/// a message's content at a point in time
// TODO: add error "latest message version cannot be deleted"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageVersion {
    pub version_id: MessageVerId,

    /// the id of who this edit. if None, this edit was made by the author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_id: Option<UserId>,

    /// the type and content of this message
    // NOTE: message type generally shouldn't change, but i don't know how to "hoist" the type field to the top level Message struct?
    #[serde(flatten)]
    pub message_type: MessageType,

    /// who this message mentioned
    #[serde(skip_serializing_if = "Mentions::is_empty")]
    pub mentions: Mentions,

    /// when this message version was created, use this as edited_at
    pub created_at: Time,

    /// when this message version was deleted
    #[serde(skip_serializing_if = "Option::is_none")]
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
                    override_name: None,
                })
            }
            m => m,
        };
        self
    }
}

/// information about a pinned message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Pinned {
    /// when this was pinned
    pub time: Time,

    /// the position of this pin. lower numbers come first.
    pub position: u16,
}

/// reorder pinned messages
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct PinsReorder {
    /// the messages to reorder
    #[serde(default)]
    #[validate(length(min = 1, max = 1024))]
    pub messages: Vec<PinsReorderItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct PinsReorderItem {
    pub id: MessageId,

    #[serde(default, deserialize_with = "some_option")]
    pub position: Option<Option<u16>>,
}

fn true_fn() -> bool {
    true
}

/// what mentions to parse from the message content. mentions will only be parsed if the message content actually contains a mention pattern.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ParseMentions {
    /// only parse mentions for these users. an empty vec disables all mentions, while None allows all mentions.
    pub users: Option<Vec<UserId>>,

    /// only parse mentions for these roles. an empty vec disables all mentions, while None allows all mentions.
    pub roles: Option<Vec<RoleId>>,

    /// whether to parse @everyone mentions from the content
    #[serde(default = "true_fn")]
    pub everyone: bool,
}

/// who/what this message notified on send
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Mentions {
    /// the users that were mentioned
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<MentionsUser>,

    /// the roles that were mentioned
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<MentionsRole>,

    /// the channels that were mentioned
    // NOTE: this may not be necessary; the user should already have all channels. this is only needed for forwards, but in that case do i really want to leak channel names/types?
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<MentionsChannel>,

    /// the custom emojis that were used in this message
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub emojis: Vec<MentionsEmoji>,

    /// if this message mentions everyone
    #[serde(default)]
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
    #[serde(rename = "type")]
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageCreate {
    /// the message's content, in either markdown or the new format depending on if use_new_text_formatting is true
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 0, max_length = 32)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 0, max = 32)))]
    #[serde(default)]
    pub attachments: Vec<MediaRef>,

    /// arbitrary metadata associated with a message
    ///
    /// deprecated: arbitrary metadata is too dubious, sorry. will come up with a better solution later
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    // TODO: remove
    pub metadata: Option<serde_json::Value>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    /// override the name of this message's sender
    ///
    /// deprecated: create new puppets for each bridged user instead
    // TODO: remove
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    #[serde(default)]
    pub override_name: Option<String>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 0, max_length = 32)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 0, max = 32)))]
    #[serde(default)]
    pub embeds: Vec<EmbedCreate>,

    /// custom timestamps (timestamp massaging), for bridge bots
    // TODO: remove (use header instead)
    pub created_at: Option<Time>,

    #[serde(default)]
    pub mentions: ParseMentions,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessagePatch {
    /// the new message content. whether its markdown/new format depends on the target message's format
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[serde(default, deserialize_with = "some_option")]
    pub content: Option<Option<String>>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 0, max_length = 32)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 0, max = 32)))]
    pub attachments: Option<Vec<MediaRef>>,

    /// arbitrary metadata associated with a message
    ///
    /// deprecated: arbitrary metadata is too dubious, sorry. will come up with a better solution later
    // TODO: remove (use header instead)
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    #[serde(default, deserialize_with = "some_option")]
    pub metadata: Option<Option<serde_json::Value>>,

    /// the message this message is replying to
    #[serde(default, deserialize_with = "some_option")]
    pub reply_id: Option<Option<MessageId>>,

    /// override the name of this message's sender
    ///
    /// deprecated: create new puppets for each bridged user instead
    // TODO: remove
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    pub override_name: Option<Option<String>>,

    pub embeds: Option<Vec<EmbedCreate>>,

    // TODO: remove (use header instead)
    pub edited_at: Option<Time>,

    pub mentions: Option<ParseMentions>,
}

// NOTE: utoipa doesnt seem to like #[deprecated] here
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageType {
    /// a basic message, using markdown
    // NOTE(v2): rename to Default
    DefaultMarkdown(MessageDefaultMarkdown),

    #[cfg(feature = "feat_message_forwarding")]
    /// (TODO) a message copied from somewhere else
    // NOTE(v2): remove
    Forward(MessageDefaultTagged),

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
    // TODO: rename to ChannelRename
    ThreadRename(MessageThreadRename),

    /// A thread was created from a message
    ThreadCreated(MessageThreadCreated),

    /// (TODO) someone mentioned this thread
    // needs some sort of antispam system. again, see github.
    // doesnt necessarily reference a thread in the same room, but usually should
    ThreadPingback(MessageThreadPingback),

    /// The channel's icon was changed
    ChannelIcon(MessageChannelIcon),
    // /// (TODO) receive announcement threads from this room
    // // but where does this get sent to???
    // RoomFollowed(MessageRoomFollowed),

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

/// Information about a message being pinned
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessagePin {
    pub pinned_message_id: MessageId,
}

/// Information about an auto moderation execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageAutomodExecution {
    /// the rules that were triggered
    pub rules: Vec<AutomodRuleStripped>,

    /// the actions that were executed
    pub actions: Vec<AutomodAction>,

    /// the content that was matched
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageThreadRename {
    #[serde(alias = "new")]
    pub name_new: String,

    #[serde(alias = "old")]
    pub name_old: String,
}

/// Information about a thread being created
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageThreadCreated {
    /// the message this thread was created from
    pub source_message_id: Option<MessageId>,
}

/// Information about the pingback
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageThreadPingback {
    pub source_room_id: RoomId,
    pub source_channel_id: ChannelId,
    pub source_user_id: UserId,
}

/// Information about a channel icon change
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageChannelIcon {
    pub icon_id_old: Option<MediaId>,
    pub icon_id_new: Option<MediaId>,
}

#[cfg(feature = "feat_message_move")]
/// Information about one or more messages being moved between threads
/// probably want this being sent in both the source and target threads, maybe
/// with a bit of different styling depending on whether its source/target
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageMember {
    pub target_user_id: UserId,
}

/// Following a room and will receive announcement posts from it
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageRoomFollowed {
    pub thread_id: ChannelId,
    pub reason: Option<String>,
}

// TODO: remove
/// audit log entries as a message (builtin moderation logging?)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageModerationLog {
    pub audit_log_entry: AuditLogEntry,
}

/// a report that moderators should look at
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageModerationReport {
    pub report: Report,
}

/// a bot command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageBotCommand {
    pub command_id: String,
}

/// a basic message, written using markdown
///
/// NOTE: new message features won't be backported here!
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageDefaultMarkdown {
    /// the message's content in markdown
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    // TODO(#325): use MediaRef here during create
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub attachments: Vec<Media>,

    /// arbitrary metadata associated with a message
    ///
    /// deprecated: arbitrary metadata is too dubious, sorry. will come up with a better solution later
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    pub metadata: Option<serde_json::Value>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub embeds: Vec<Embed>,

    /// override the name of this message's sender
    ///
    /// deprecated: create new puppets for each bridged user instead
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    pub override_name: Option<String>,
    // // experimental! don't touch yet.
    // #[cfg(feature = "feat_interaction")]
    // #[cfg_attr(feature = "utoipa", schema(ignore))]
    // #[serde(default)]
    // pub interactions: Interactions,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageCall {
    /// when the call ended. is None if the call is still going.
    pub ended_at: Option<Time>,

    /// the people who joined the call
    pub participants: Vec<UserId>,
}

// TODO: remove
/// ways to interact with a message
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Interactions {
    #[cfg(feature = "feat_interaction_reaction")]
    /// show placeholder reactions (they appear with zero total reactions) for these emoji
    pub reactions_default: Option<Vec<ReactionKey>>,
    // for message create
    // pub reactions_default: Option<Vec<ReactionKeyParam>>,

    // yet another rabbit hole. not worth it for now.
    #[cfg(feature = "feat_interaction_status")]
    #[serde(flatten)]
    pub status: Option<InteractionStatus>,
}

/// the current status
#[cfg(feature = "feat_interaction_reaction")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageMigrate {
    /// which messages to move
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 128)))]
    pub message_ids: Vec<MessageId>,

    /// must be in same room (for now...)
    pub target_id: ChannelId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageModerate {
    /// which messages to delete
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub delete: Vec<MessageId>,

    /// which messages to remove
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub remove: Vec<MessageId>,

    /// which messages to restore
    #[serde(default)]
    #[cfg_attr(feature = "validator", validate(length(max = 128)))]
    pub restore: Vec<MessageId>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RepliesQuery {
    /// how deeply to fetch replies
    #[serde(default = "fn_one")]
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 8)))]
    pub depth: u16,

    /// how many replies to fetch per branch
    pub breadth: Option<u16>,

    /// which parent message to fetch replies from, where 0 is the message itself, 1 is its parent, and so on.
    pub context: Option<u16>,
}

/// always returns one
fn fn_one() -> u16 {
    1
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct ContextQuery {
    pub to_start: Option<MessageId>,
    pub to_end: Option<MessageId>,
    pub limit: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RatelimitPut {
    #[serde(default, deserialize_with = "some_option")]
    pub slowmode_thread_expire_at: Option<Option<Time>>,

    #[serde(default, deserialize_with = "some_option")]
    pub slowmode_message_expire_at: Option<Option<Time>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ContextResponse {
    pub items: Vec<MessageV2>,
    pub total: u64,
    pub has_after: bool,
    pub has_before: bool,
}

impl Diff<Message> for MessagePatch {
    fn changes(&self, other: &Message) -> bool {
        match &other.latest_version.message_type {
            MessageType::DefaultMarkdown(m) => {
                self.content.changes(&m.content)
                    || self.reply_id.changes(&m.reply_id)
                    || self.embeds.is_some()
                    || self.attachments.is_some()
                    || self.reply_id.changes(&m.reply_id)
                    || self.override_name.changes(&m.override_name)
                    || self.attachments.as_ref().is_some_and(|a| {
                        a.len() != m.attachments.len()
                            || a.iter().zip(&m.attachments).any(|(a, b)| a.id != b.id)
                    })
            }
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
            MessageType::ThreadRename(_) => false,
            MessageType::ThreadPingback(_) => true,
            MessageType::ChannelIcon(_) => false,
            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(_) => false,
            MessageType::Call(_) => false,
            MessageType::ThreadCreated(_) => false,

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
            MessageType::ThreadRename(_) => true,
            MessageType::ThreadPingback(_) => true,
            MessageType::ChannelIcon(_) => true,
            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(_) => false,
            MessageType::Call(_) => false,
            MessageType::ThreadCreated(_) => false,
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
