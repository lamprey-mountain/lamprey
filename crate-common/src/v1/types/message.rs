use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

#[cfg(feature = "feat_automod")]
use crate::v1::types::RedexId;

use crate::v1::types::emoji::Emoji;
use crate::v1::types::moderation::Report;
use crate::v1::types::reaction::ReactionCounts;
use crate::v1::types::util::some_option;
use crate::v1::types::util::Diff;
use crate::v1::types::util::Time;
use crate::v1::types::RoomId;
use crate::v1::types::{
    AuditLogEntry, Embed, Role, RoleId, Room, RoomMember, Thread, ThreadMember, UserId,
};

use super::EmbedCreate;
use super::{
    media::{Media, MediaRef},
    MessageId, MessageVerId, ThreadId, User,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Message {
    #[serde(flatten)]
    pub message_type: MessageType,
    pub id: MessageId,
    pub thread_id: ThreadId,
    pub version_id: MessageVerId,

    /// unique string sent by the client to identify this message
    /// maybe i will replace with a header so nonces can be used everywhere
    pub nonce: Option<String>,

    /// the id of who sent this message
    pub author_id: UserId,

    pub mentions: Mentions,

    /// exists if this message is pinned
    pub pinned: Option<Pinned>,

    #[serde(default)]
    pub reactions: ReactionCounts,

    // pub moved_at: Option<Time>,
    // pub moved_from: Option<(ThreadId, MessageId)>,
    pub created_at: Option<Time>,

    /// deleted messages can still be viewed by moderators for a period of time, but otherwise cannot be recovered
    pub deleted_at: Option<Time>,

    /// removed messages are hidden for non moderators. they are recoverable by moderators
    pub removed_at: Option<Time>,

    pub edited_at: Option<Time>,
    // // drop the is_?
    // pub is_ephemeral: bool,
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

/// who/what this message notified on send
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Mentions {
    pub users: Vec<UserId>,
    pub roles: Vec<RoleId>,
    pub threads: Vec<ThreadId>,
    pub rooms: Vec<ThreadId>,

    /// if this mentioned everyone in the room
    pub all_in_room: bool,

    /// if this mentioned everyone in the thread
    pub all_in_thread: bool,
}

/// data that has been resolved from the ids, provided on request
// maybe don't put it in messages, this could be useful elsewhere
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Resolved {
    pub users: Vec<User>,
    pub room_members: Vec<RoomMember>,
    pub thread_members: Vec<ThreadMember>,
    pub roles: Vec<Role>,
    pub rooms: Vec<Room>,
    pub threads: Vec<Thread>,
    pub messages: Vec<Message>,
    // pub emoji: Vec<Emoji>,
}

// /// resolve the final profile details for a user (after overrides)
// #[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct ResolvedProfile {
//     id: UserId,
//     name: String,
//     description: Option<String>,
//     avatar: Option<MediaId>,
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub metadata: Option<serde_json::Value>,

    /// the message this message is replying to
    pub reply_id: Option<MessageId>,

    /// override the name of this message's sender
    ///
    /// deprecated: create new puppets for each bridged user instead
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    #[serde(default)]
    pub override_name: Option<String>,

    /// used so the client can know if the message was sent or not
    ///
    /// deprecated: Ideompotency-Key
    // TODO(#87): actually support Ideompotency-Key
    // TODO(#246): use this to deduplicate messages
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    #[serde(default)]
    pub nonce: Option<String>,

    #[cfg(feature = "feat_custom_embeds")]
    #[serde(default)]
    pub embeds: Vec<EmbedCreate>,

    /// custom timestamps (timestamp massaging), for bridge bots
    pub created_at: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    #[serde(default, deserialize_with = "some_option")]
    pub metadata: Option<Option<serde_json::Value>>,

    /// the message this message is replying to
    #[serde(default, deserialize_with = "some_option")]
    pub reply_id: Option<Option<MessageId>>,

    /// override the name of this message's sender
    ///
    /// deprecated: create new puppets for each bridged user instead
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    pub override_name: Option<Option<String>>,

    #[cfg(feature = "feat_custom_embeds")]
    pub embeds: Option<Vec<EmbedCreate>>,

    pub edited_at: Option<Time>,
}

// NOTE: utoipa doesnt seem to like #[deprecated] here
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageType {
    /// a basic message, using markdown
    DefaultMarkdown(MessageDefaultMarkdown),

    #[cfg(feature = "feat_message_forwarding")]
    /// (TODO) a message copied from somewhere else
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

    /// (TODO) call ended in a dm/gdm
    Call(MessageCall),

    /// this thread was renamed
    ThreadRename(MessageThreadRename),

    /// (TODO) someone mentioned this thread
    // needs some sort of antispam system. again, see github.
    // doesnt necessarily reference a thread in the same room, but usually should
    ThreadPingback(MessageThreadPingback),
    // /// (TODO) receive announcement threads from this room
    // // but where does this get sent to???
    // RoomFollowed(MessageRoomFollowed),

    // /// (TODO) interact with a bot, uncertain if i'll go this route
    // BotCommand(MessageBotCommand),

    // /// (TODO) implement some sort of automoderator? uncertain
    // #[cfg(feature = "feat_automod")]
    // ModerationAuto(MessageModerationAuto),

    // /// (TODO) implement a reporting system? uncertain (reports are certain, but reports-as-messages vs as-threads idk)
    // // #[deprecated = "reports will be impl'd as threads"]
    // ModerationReport(MessageModerationReport),

    // Nudge,
}

/// Information about a message being pinned
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessagePin {
    pub pinned_message_id: MessageId,
}

/// Information about a thread being renamed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageThreadRename {
    #[serde(alias = "new")]
    pub name_new: String,

    #[serde(alias = "old")]
    pub name_old: String,
}

/// Information about the pingback
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageThreadPingback {
    pub source_room_id: RoomId,
    pub source_thread_id: ThreadId,
    pub source_user_id: UserId,
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
    pub source_id: ThreadId,
    pub target_id: ThreadId,
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
    pub thread_id: ThreadId,
    pub reason: Option<String>,
}

/// audit log entries as a message (builtin moderation logging?)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageModerationLog {
    pub audit_log_entry: AuditLogEntry,
}

/// automatic moderation reports
#[cfg(feature = "feat_automod")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageModerationAuto {
    pub redex_id: RedexId,
    pub audit_log_entries: Vec<AuditLog>,
    pub context: Vec<AutomodContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema), schema(no_recursion))]
#[serde(tag = "type", content = "data")]
pub enum AutomodContext {
    Message(Message),
    User(User),
    ThreadMember(ThreadMember),
    RoomMember(RoomMember),
    Thread(Thread),
    Media(Media),
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

/// ways to interact with a message
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Interactions {
    #[cfg(feature = "feat_interaction_reaction")]
    /// show placeholder reactions (they appear with zero total reactions) for these emoji
    pub reactions_default: Option<Vec<Emoji>>,

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

impl Diff<Message> for MessagePatch {
    fn changes(&self, other: &Message) -> bool {
        match &other.message_type {
            MessageType::DefaultMarkdown(m) => {
                self.content.changes(&m.content)
                    || self.metadata.changes(&m.metadata)
                    || self.reply_id.changes(&m.reply_id)
                    || self.override_name.changes(&m.override_name)
                    || self.embeds.is_some()
                    || self.attachments.as_ref().is_some_and(|a| {
                        a.len() != m.attachments.len()
                            || a.iter().zip(&m.attachments).any(|(a, b)| a.id != b.id)
                    })
            }
            // this edit is invalid!
            _ => false,
        }
    }
}

impl MessageType {
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
            #[cfg(feature = "feat_automod")]
            MessageType::ModerationAuto(_) => true,
            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(_) => false,
            MessageType::Call(_) => false,
        }
    }

    pub fn is_editable(&self) -> bool {
        matches!(self, MessageType::DefaultMarkdown(_))
    }

    pub fn is_movable(&self) -> bool {
        matches!(self, MessageType::DefaultMarkdown(_))
    }
}
