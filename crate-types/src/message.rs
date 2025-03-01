use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::emoji::Emoji;
use crate::moderation::Report;
use crate::util::some_option;
use crate::util::Diff;
use crate::RedexId;
use crate::{
    AuditLog, Role, RoleId, Room, RoomMember, Thread, ThreadCreateRequest, ThreadMember,
    ThreadPatch, UrlEmbed, UserId,
};

use super::{Media, MediaRef, MessageId, MessageVerId, ThreadId, User};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Message {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    // #[serde(flatten)]
    // pub message_type: MessageType2,
    pub id: MessageId,
    pub thread_id: ThreadId,
    pub version_id: MessageVerId,

    /// unique string sent by the client to identify this message
    /// maybe i will replace with a header so nonces can be used everywhere
    pub nonce: Option<String>,

    // #[deprecated = "not that useful"]
    pub ordering: i32,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub attachments: Vec<Media>,

    // matrix does this and its pretty useful for bots, but idk if its a good idea to always have this...
    // #[deprecated = "arbitrary metadata is too dubious, sorry. will come up with a better solution later."]
    /// arbitrary metadata attached to this message
    pub metadata: Option<serde_json::Value>,

    // TODO: replying to multiple messages at once? might be useful, needs ui design
    pub reply_id: Option<MessageId>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub embeds: Vec<UrlEmbed>,

    // #[deprecated = "create new puppets for each unique combination"]
    // new puppets every time may be expensive for some use cases, but eh idk
    /// override the name of the user who sent this message. will probably be remved soon!
    pub override_name: Option<String>,

    /// who sent this message
    #[deprecated = "use author_id and fetch manually, better caching and easier server impl"]
    pub author: User,

    // /// the id of who sent this message
    // pub author_id: User,

    // #[deprecated = "use message.state"]
    pub is_pinned: bool,
    // #[serde(flatten)]
    // pub mentions: Mentions,

    // #[serde(flatten)]
    // pub state: MessageState,
}

/// lifecycle of a message
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "state")]
pub enum MessageState {
    #[default]
    /// default state that new messages
    Default,

    /// message is pinned to the thread
    Pinned {
        pin_order: u32,
        pin_at: time::OffsetDateTime,
    },

    /// message is not stored
    Ephemeral,

    // /// message was moved from another thread
    // Moved { move_info: MessageId },

    // /// message was moved from another thread
    // Copied { move_info: MessageId, source_id: MessageId },
    /// will be permanently deleted soon, visible to moderators
    Deleted,
}

/// who/what this message notified on send
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Mentions {
    pub mentions_users: Vec<UserId>,
    pub mentions_roles: Vec<RoleId>,
    pub mentions_threads: Vec<ThreadId>,
    pub mentions_rooms: Vec<ThreadId>,

    /// if this mentioned everyone in the room
    pub mentions_all_in_room: bool,

    /// if this mentioned everyone in the thread
    pub mentions_all_in_thread: bool,
}

/// data that has been resolved from the ids, provided on request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Resolved {
    pub users: Vec<User>,
    pub room_members: Vec<RoomMember>,
    pub thread_members: Vec<ThreadMember>,
    pub roles: Vec<Role>,
    pub rooms: Vec<Room>,
    pub threads: Vec<Thread>,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageCreateRequest {
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

    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<MessageId>,
    /// temporary?
    pub override_name: Option<String>,
    pub nonce: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessagePatch {
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

    #[serde(default, deserialize_with = "some_option")]
    pub metadata: Option<Option<serde_json::Value>>,

    #[serde(default, deserialize_with = "some_option")]
    pub reply_id: Option<Option<MessageId>>,

    // is this temporary, or should i keep it?
    // removing it would break all existing bridged messages
    #[serde(default, deserialize_with = "some_option")]
    pub override_name: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MessageType {
    /// a basic message
    Default,

    /// a message logging an update to the thread
    ThreadUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename = "type")]
pub enum MessageType2 {
    /// a basic message, using the legacy markdown syntax
    DefaultMarkdown(MessageDefaultMarkdown),

    /// a basic message, using the new tagged text format
    DefaultTagged(MessageDefaultTagged),

    /// a message copied from somewhere else
    Forward(MessageDefaultTagged),

    MessagePinned(MessagePin),
    MessageUnpinned(MessagePin),
    MemberAdd(MessageMember),
    MemberRemove(MessageMember),

    /// a message logging an update to the thread
    ThreadUpdate(ThreadPatch),
    ThreadCreate(ThreadCreateRequest),

    RoomFollowed(MessageRoomFollowed),

    BotCommand(MessageBotCommand),

    ModerationLog(MessageModerationLog),
    ModerationAuto(MessageModerationAuto),
    ModerationReport(MessageModerationReport),

    SystemMessage(MessageSystemMessage),
}

/// Information about a message being pinned or unpinned
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessagePin {
    pub message_id: MessageId,
    pub user_id: UserId,
    pub reason: Option<String>,
}

/// Information about a member being added or removed from a thread
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageMember {
    pub target_user_id: UserId,
    pub actor_user_id: UserId,
    pub reason: Option<String>,
}

/// Following a room and will receive announcement posts from it
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageRoomFollowed {
    pub thread_id: ThreadId,
    pub user_id: UserId,
    pub reason: Option<String>,
}

/// audit log entries as a message (builtin moderation logging?)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageModerationLog {
    pub audit_log_entry: AuditLog,
}

/// automatic moderation reports
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageModerationAuto {
    pub redex_id: RedexId,
    pub audit_log_entries: Vec<AuditLog>,
    pub context: Vec<AutomodContext>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageModerationReport {
    pub report: Report,
}

/// a bot command
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageBotCommand {
    pub command_id: String,
}

/// a message (announcement? motd?) from the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageSystemMessage {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub attachments: Vec<Media>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub embeds: Vec<UrlEmbed>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Reactions {
    pub emoji: Emoji,
    pub count: u64,

    #[serde(rename = "self")]
    pub self_reacted: bool,
}

/// a basic message, using the legacy markdown syntax
// #[deprecated = "markdown is GONE, baby! long live markdown!"]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageDefaultMarkdown {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub attachments: Vec<Media>,

    #[deprecated = "arbitrary metadata is too dubious, sorry. will come up with a better solution later."]
    pub metadata: Option<serde_json::Value>,

    pub reply_id: Option<MessageId>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub embeds: Vec<UrlEmbed>,
}

/// a basic message, using the shiny new and very experimental tagged text format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageDefaultTagged {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub attachments: Vec<Media>,

    #[deprecated = "arbitrary metadata is too dubious, sorry. will come up with a better solution later."]
    pub metadata: Option<serde_json::Value>,

    pub reply_id: Option<MessageId>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 32))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32), nested))]
    pub embeds: Vec<UrlEmbed>,

    pub reactions: Reactions,
}

// mod v {
//     impl Validate for MessageType2 {}
// }

/// a basic message, using the shiny new and very experimental tagged text format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Interactions {
    /// show placeholder reactions (they appear with zero total reactions) for these emoji
    pub reactions_default: Vec<Emoji>,
}

impl Diff<Message> for MessagePatch {
    fn changes(&self, other: &Message) -> bool {
        self.content.changes(&other.content)
            || self.metadata.changes(&other.metadata)
            || self.reply_id.changes(&other.reply_id)
            || self.override_name.changes(&other.override_name)
            || self.attachments.as_ref().is_some_and(|a| {
                a.len() != other.attachments.len()
                    || a.iter().zip(&other.attachments).any(|(a, b)| a.id != b.id)
            })
    }
}

impl MessageType {
    pub fn is_deletable(&self) -> bool {
        match self {
            MessageType::Default => true,
            MessageType::ThreadUpdate => false,
        }
    }

    pub fn is_editable(&self) -> bool {
        match self {
            MessageType::Default => true,
            MessageType::ThreadUpdate => false,
        }
    }
}
