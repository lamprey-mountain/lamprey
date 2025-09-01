use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{util::Time, MessageId, NotificationId, ThreadId};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotificationCreate {
    pub thread_id: ThreadId,
    pub message_id: MessageId,
    pub added_at: Option<Time>, // set in the future to create a reminder
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
