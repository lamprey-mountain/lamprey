use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    util::Time, Message, MessageId, NotificationId, PaginationResponse, Room, RoomId, Thread,
    ThreadId,
};

/// how to handle an event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotifAction {
    /// Notifications are sent when this event happens
    Notify,

    /// Notifications are added to the inbox when this event happens
    Watching,

    /// This event is ignored entirely
    Ignore,
}

/// notification config for a user (works globally)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotifsGlobal {
    pub mute: Option<Mute>,
    pub messages: NotifAction,
    pub mentions: NotifAction,
    pub threads: NotifAction,
    pub room_public: NotifAction,
    pub room_private: NotifAction,
    pub room_dm: NotifAction,
}

/// notification config for a room
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotifsRoom {
    pub mute: Option<Mute>,
    pub messages: Option<NotifAction>,
    pub mentions: Option<NotifAction>,
    pub threads: Option<NotifAction>,
}

/// notification config for a thread
// how do i deal with different thread types
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotifsThread {
    pub mute: Option<Mute>,
    pub messages: Option<NotifAction>,
    pub mentions: Option<NotifAction>,
}

/// notification config for a message
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotifsMessage {
    pub mute: Option<Mute>,
    pub replies: Option<NotifAction>,
    // pub edits: Option<NotifAction>,
    // pub reactions: Option<NotifAction>,
}

/// how long to mute for
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Mute {
    pub expires_at: Option<Time>,
}

/// a notification; a unit of stuff that may show up in your inbox or be pushed to you
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Notification {
    pub id: NotificationId,

    /// the thread this message was sent in
    pub thread_id: ThreadId,

    /// the id of the message that was sent
    pub message_id: MessageId,

    /// why this was created
    pub reason: NotificationReason,

    /// when this was added to the inbox
    pub added_at: Time,
}

// in order of precedence
/// what caused this notification to be created
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotificationReason {
    /// user manually added this notification
    Reminder,

    /// this message mentioned you. overrides MentionBulk
    Mention,

    /// this message mentioned @room, @thread, or roles
    MentionBulk,

    /// this message replied to one of your own messages
    Reply,
    // /// this is a new thread
    // ThreadNew,

    // /// this thread is unread
    // /// message_id wont have any meaning, client should fetch context instead
    // ThreadUnread,
}

// enum NotificationReasonMessage {
//     Manual,
//     Mention,
//     MentionBulk,
//     Reply,
//     Unread,
// }

// enum NotificationReasonThread {
//     #[serde(flatten)]
//     Message(NotificationReasonMessage),
//     New,
// }

impl Default for NotifsGlobal {
    fn default() -> Self {
        NotifsGlobal {
            mute: None,
            messages: NotifAction::Watching,
            mentions: NotifAction::Notify,
            threads: NotifAction::Watching,
            room_public: NotifAction::Watching,
            room_private: NotifAction::Watching,
            room_dm: NotifAction::Watching,
        }
    }
}

// new types below; still a work in progress!

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct InboxListParams {
    /// only include notifications from these rooms
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32))]
    pub room_id: Vec<RoomId>,

    /// only include notifications from these threads
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32))]
    pub thread_id: Vec<ThreadId>,

    /// include messages marked as read too
    #[serde(default)]
    pub include_read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotificationCreate {
    /// the thread this message was sent in
    pub thread_id: ThreadId,

    /// the id of the message that was sent
    pub message_id: MessageId,

    /// set this in the future to create a reminder
    pub added_at: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct NotificationMarkRead {
    /// mark these messages as read
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub message_ids: Vec<MessageId>,

    /// mark everything in these threads as read
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub thread_ids: Vec<ThreadId>,

    /// mark everything in these rooms as read
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub room_ids: Vec<RoomId>,

    /// mark everything as read
    #[serde(default)]
    pub everything: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct NotificationFlush {
    /// restrict to just notifications before (including) this message id
    pub before: Option<MessageId>,

    /// restrict to just notifications after (including) this message id
    pub after: Option<MessageId>,

    /// restrict to just these messages
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub message_ids: Option<Vec<MessageId>>,

    /// restrict to just these threads
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub thread_ids: Option<Vec<ThreadId>>,

    /// restrict to just these rooms
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub room_ids: Option<Vec<RoomId>>,

    /// also include unread notifications
    #[serde(default)]
    pub include_unread: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct InboxThreadsParams {
    /// the order to return inbox threads in
    pub order: InboxThreadsOrder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum InboxThreadsOrder {
    /// most active threads first (order by last_version_id desc)
    Activity,

    /// last active threads first (order by last_version_id asc)
    // NOTE: not sure how useful this is, but including for completeness
    Inactivity,

    /// most recently created threads first (order by id desc)
    Newest,

    /// most recently created threads first (order by id desc)
    Oldest,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotificationPagination {
    #[serde(flatten)]
    pub inner: PaginationResponse<Notification>,
    pub threads: Vec<Thread>,
    pub messages: Vec<Message>,
    pub rooms: Vec<Room>,
}
