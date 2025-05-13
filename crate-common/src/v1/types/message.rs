use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

#[allow(unused_imports)]
#[cfg(feature = "feat_reactions")]
use crate::v1::types::emoji::Emoji;

#[allow(unused_imports)]
#[cfg(feature = "feat_reactions")]
use crate::v1::types::reaction::ReactionCounts;

#[cfg(feature = "feat_automod")]
use crate::v1::types::RedexId;

use crate::v1::types::moderation::Report;
use crate::v1::types::util::some_option;
use crate::v1::types::util::Diff;
use crate::v1::types::util::Time;
use crate::v1::types::RoomId;
use crate::v1::types::{
    AuditLog, Role, RoleId, Room, RoomMember, Thread, ThreadMember, ThreadPatch, UrlEmbed, UserId,
};

use super::{
    media::{Media, MediaRef},
    MessageId, MessageVerId, ThreadId, User,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// the index/position/ordering of this message in the thread
    ///
    /// deprecated: not that useful
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    pub ordering: i32,

    /// the id of who sent this message
    pub author_id: UserId,

    /// if this message is pinned
    ///
    /// deprecated: use message.state
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    pub is_pinned: bool,

    pub mentions: Mentions,

    #[serde(flatten)]
    pub state: MessageState,
    pub state_updated_at: Time,
    // pub pinned_at: Option<Time>,
    // pub pinned_order: Option<u8>,
    // pub moved_at: Option<Time>,
    // pub moved_from: Option<(ThreadId, MessageId)>,
    // pub deleted_at: Option<Time>,
    // // drop the is_?
    // pub is_ephemeral: bool,
}

// /// unlisted + content is stripped
// struct Deleted {
//     at: Option<Time>,
//     by: Option<UserId>,
//     reason: Option<String>,
//     is_undeletable: bool,
// }

// struct Pinned {
//     at: Time,
//     order: u8, // tiebreak by id
// }

// struct Moved {
//     at: Time,
//     from_thread: Option<ThreadId>, // removed if thread is deleted
//     from_message: Option<MessageId>, // removed if thread or message is deleted
// }

/// lifecycle of a message
// TODO: switch back to fields, this is pretty h
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "state")]
pub enum MessageState {
    #[default]
    /// default state that new messages
    Default,

    /// message is pinned to the thread
    Pinned { pin_order: u32 },

    /// (TODO) message is not stored
    Ephemeral,

    #[cfg(feature = "feat_message_move")]
    /// message was moved from another thread
    Moved {
        /// the relevant MessagesMoved message in this thread
        move_info: MessageId,
    },

    /// will be permanently deleted soon, visible to moderators for now
    Deleted,
}

/// who/what this message notified on send
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageCreate {
    /// the message's content, in either markdown or the new format depending on if use_new_text_formatting is true
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

    /// uses the new new formatting system if true, otherwise uses markdown
    // TEMP: opt in to the new formatting system
    #[serde(default)]
    pub use_new_text_formatting: bool,

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
    pub embeds: Vec<UrlEmbed>,
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
    pub embeds: Option<Vec<UrlEmbed>>,
}

// FIXME: utoipa doesnt seem to like #[deprecated] here
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageType {
    /// a basic message, using the legacy markdown syntax
    /// previously called "Default"!
    // NOTE: i don't know how soon i want to commit to the new format - there
    // might be some rough edges, we'll see. i'll support markdown for at least
    // a while.
    // #[deprecated = "use the new text format"]
    DefaultMarkdown(MessageDefaultMarkdown),

    #[cfg(feature = "feat_message_new_text")]
    // #[deprecated = "didn't work how i thought it would"]
    /// a basic message, using the new tagged text format
    DefaultTagged(MessageDefaultTagged),

    #[cfg(feature = "feat_message_forwarding")]
    /// (TODO) a message copied from somewhere else
    Forward(MessageDefaultTagged),

    /// (TODO) a message was pinned
    MessagePinned(MessagePin),

    /// (TODO) a message was unpinned
    MessageUnpinned(MessagePin),

    #[cfg(feature = "feat_message_move")]
    /// (TODO) one or more messages were moved
    MessagesMoved(MessagesMoved),

    /// (TODO) a member was added to the thread (what about room?)
    MemberAdd(MessageMember),

    /// (TODO) a member was removed from the thread (what about room?)
    MemberRemove(MessageMember),

    // /// call ended (duration, participants)
    // CallEnd(MessagesCallEnd),
    /// a message logging an update to the thread
    ThreadUpdate(MessageThreadUpdate),

    // why have a separate event instead of ThreadUpdate? semantics i guess
    /// (TODO) a message at the beginning of a thread
    ThreadCreate(MessageThreadUpdate),

    /// someone mentioned this thread
    // needs some sort of antispam system. again, see github.
    // doesnt necessarily reference a thread in the same room, but usually should
    ThreadPingback(MessageThreadPingback),

    /// (TODO) receive announcement threads from this room
    // but where does this get sent to???
    RoomFollowed(MessageRoomFollowed),

    /// (TODO) interact with a bot, uncertain if i'll go this route
    BotCommand(MessageBotCommand),

    /// (TODO) repost audit log to a thread? uncertain
    // ...or display the audit log as a thread
    // #[deprecated = "use audit log"]
    ModerationLog(MessageModerationLog),

    /// (TODO) implement some sort of automoderator? uncertain
    #[cfg(feature = "feat_automod")]
    ModerationAuto(MessageModerationAuto),

    /// (TODO) implement a reporting system? uncertain (reports are certain, but reports-as-messages vs as-threads idk)
    // #[deprecated = "reports will be impl'd as threads"]
    ModerationReport(MessageModerationReport),

    /// (TODO) important message from the system/server
    // #[deprecated = "check if user.type is System"]
    SystemMessage(MessageSystemMessage),
    // Nudge,
}

/// Information about a message being pinned or unpinned
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessagePin {
    pub message_id: MessageId,
    pub user_id: UserId,
    pub reason: Option<String>,
}

/// Information about a thread being updated
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageThreadUpdate {
    pub patch: ThreadPatch,
}

/// Information about the pingback
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[cfg(feature = "feat_automod")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageModerationAuto {
    pub redex_id: RedexId,
    pub audit_log_entries: Vec<AuditLog>,
    pub context: Vec<AutomodContext>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// a basic message, using the legacy markdown syntax
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
    pub embeds: Vec<UrlEmbed>,

    /// override the name of this message's sender
    ///
    /// deprecated: create new puppets for each bridged user instead
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    pub override_name: Option<String>,

    #[cfg(feature = "feat_reactions")]
    #[serde(default)]
    pub reactions: ReactionCounts,
}

/// a basic message, using the shiny new and very experimental tagged text format
#[cfg(feature = "feat_message_new_text")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct MessageDefaultTagged {
    /// the message's content in the new format
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub content: Option<String>,

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
    pub embeds: Vec<UrlEmbed>,

    #[cfg(feature = "feat_reactions")]
    #[serde(default)]
    pub reactions: ReactionCounts,

    // experimental! don't touch yet.
    #[cfg(feature = "feat_interaction")]
    #[cfg_attr(feature = "utoipa", schema(ignore))]
    #[serde(default)]
    pub interactions: Interactions,
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
                    || self.embeds.changes(&m.embeds)
                    || self.attachments.as_ref().is_some_and(|a| {
                        a.len() != m.attachments.len()
                            || a.iter().zip(&m.attachments).any(|(a, b)| a.id != b.id)
                    })
            }
            #[cfg(feature = "feat_message_new_text")]
            MessageType::DefaultTagged(m) => {
                self.content.changes(&m.content)
                    || self.metadata.changes(&m.metadata)
                    || self.reply_id.changes(&m.reply_id)
                    || self.embeds.changes(&m.embeds)
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
            #[cfg(feature = "feat_message_new_text")]
            MessageType::DefaultTagged(_) => true,
            #[cfg(feature = "feat_message_forwarding")]
            MessageType::Forward(_) => true,
            MessageType::MessagePinned(_) => true,
            MessageType::MessageUnpinned(_) => true,
            MessageType::MemberAdd(_) => false,
            MessageType::MemberRemove(_) => false,
            MessageType::ThreadUpdate(_) => false,
            MessageType::ThreadCreate(_) => false,
            MessageType::RoomFollowed(_) => true,
            MessageType::BotCommand(_) => true,

            // these ones probably need special permission checks
            MessageType::ThreadPingback(_) => true,
            MessageType::ModerationLog(_) => true,
            #[cfg(feature = "feat_automod")]
            MessageType::ModerationAuto(_) => true,
            MessageType::ModerationReport(_) => true,
            MessageType::SystemMessage(_) => true,

            #[cfg(feature = "feat_message_move")]
            MessageType::MessagesMoved(_) => false,
        }
    }

    pub fn is_editable(&self) -> bool {
        #[cfg(feature = "feat_message_new_text")]
        return matches!(
            self,
            MessageType::DefaultMarkdown(_) | MessageType::DefaultTagged(_)
        );

        #[cfg(not(feature = "feat_message_new_text"))]
        matches!(self, MessageType::DefaultMarkdown(_))
    }
}

// impl MessagePatch {
//     pub fn can_append(&self, other: &Message) -> bool {
//         if !self.changes(other) {
//             return true;
//         }
//         match &other.message_type {
//             MessageType::DefaultMarkdown(m) => {
//                 if let Some(c) = &self.content {
//                     let ok = match (&m.content, c) {
//                         (None, None) => true,
//                         (None, Some(_)) => true,
//                         (Some(_), None) => false,
//                         (Some(a), Some(b)) => a.starts_with(b.as_str()),
//                     };
//                     if !ok {
//                         return false;
//                     }
//                 }
//                 if self.metadata.changes(&m.metadata)
//                     || self.reply_id.changes(&m.reply_id)
//                     || self.override_name.changes(&m.override_name)
//                     || self.embeds.changes(&m.embeds)
//                     || self.attachments.as_ref().is_some_and(|a| {
//                         a.len() != m.attachments.len()
//                             || a.iter().zip(&m.attachments).any(|(a, b)| a.id != b.id)
//                     })
//                 {
//                     return false;
//                 }
//                 true
//             }
//             #[cfg(feature = "feat_message_new_text")]
//             MessageType::DefaultTagged(m) => todo!(),
//             _ => false,
//         }
//     }
// }
