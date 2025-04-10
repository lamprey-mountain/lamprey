use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{util::Time, MessageId, RoomId, ThreadId};

/// a bunch of random ideas from past/old projects that i might reuse
#[allow(unused)]
mod old {
    // TODO: pare/reduce these down until i get somewhat decent types
    use crate::v1::types::{util::Time, MessageId, MessageVerId, RoomId, ThreadId};

    enum NotificationLevelGlobal {
        /// You will be notified of new replies in threads.
        Replies,

        /// You will be notified of new threads.
        Creation,

        /// New threads and thread updates show up in your inbox.
        Watching,

        /// You will only be notified on @mention
        Mentions,
    }

    enum NotificationLevelRoom {
        /// Uses your global default notification config
        Default,

        /// You will be notified of new replies in threads
        Replies,

        /// You will be notified of new threads
        Creation,

        /// New threads and thread updates show up in your inbox
        Watching,

        /// You will only be notified on @mention
        Mentions,

        /// This thread does not create any notifications
        /// This setting overrides any thread specific level
        Muted { until: Option<Time> },
    }

    enum NotificationLevelThread {
        /// Uses the room's default notifications
        Default,

        /// You will be notified of new replies in this thread
        Replies,

        /// Updates to this thread will show up in your inbox
        Watching,

        /// You will only be notified on @mention
        Mentions,

        /// This thread does not create any notifications
        Muted { until: Option<Time> },
    }

    /// the naive solution?
    enum Setting {
        Default,

        /// notify on all new threads + all messages in watched threads
        ThreadsAndEverything,

        /// notify on all new threads + all mentions in watched threads
        ThreadsAndMentions,

        /// notify on all messages in watched threads
        Everything,

        /// notify on all mentions in watched threads (a good default)
        Mentions,

        /// don't notify
        Subdued,
        Muted,
    }

    /// the better solution?
    enum RoomSetting {
        Default,

        /// notify on new threads
        Everything,

        /// notify on all new voice threads (for dm calls?)
        Voice,

        /// don't notify on new threads (a good default)
        Mentions,

        /// don't notify
        Subdued,
        Muted,
    }

    enum ThreadSetting {
        Default,

        /// notify on all new messages (also a good default?)
        Everything,

        /// notify on all mentions (default?)
        Mentions,

        /// don't notify
        Muted,
    }

    /// another solution? (i prefer this one)
    struct RoomSettings {
        /// notify when any new thread is created
        notify_on_thread: bool,
        // notify_on_thread: None | VoiceOnly | All,
        /// notify when any new message is created
        notify_on_message: bool,

        /// don't receive notifications
        mute: MuteOptions,
    }

    struct MuteOptions {
        /// should this fully hide any mention ui
        full: bool,

        /// how long to mute for
        duration: MuteDuration,
    }

    enum MuteDuration {
        Forever,
        Until(u64),
    }
}

/// how to handle an event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotifAction {
    /// Notifications are created when this event happens
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
    pub mute: Option<MuteDuration>,
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
    pub mute: Option<MuteDuration>,
    pub messages: Option<NotifAction>,
    pub mentions: Option<NotifAction>,
    pub threads: Option<NotifAction>,
}

/// notification config for a thread
// how do i deal with different thread types
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotifsThread {
    pub mute: Option<MuteDuration>,
    pub messages: Option<NotifAction>,
    pub mentions: Option<NotifAction>,
}

/// notification config for a message
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotifsMessage {
    pub mute: Option<MuteDuration>,
    pub replies: Option<NotifAction>,
    // pub edits: Option<NotifAction>,
    // pub reactions: Option<NotifAction>,
}

/// how long to mute for
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MuteDuration {
    Forever,

    #[serde(untagged)]
    Until(Time),
}

/// a notification; a unit of stuff that may show up in your inbox or be pushed to you
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Notification {
    #[serde(flatten)]
    pub info: NotificationInfo,

    /// when this was read
    pub read_at: Option<Time>,

    /// when this notification was created
    /// can be set in the future to create a reminder
    pub added_at: Time,
    // bookmarks? how do they behave differently?
    // pub is_bookmark: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum NotificationInfo {
    Thread {
        room_id: RoomId,
        thread_id: ThreadId,
        reason: NotificationReasonThread,
        // summary: Summary,
    },
    Message {
        room_id: RoomId,
        thread_id: ThreadId,
        message_id: MessageId,
        reason: NotificationReasonMessage,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct InboxPatch {
    /// mark notifications as read
    #[serde(default)]
    pub mark_read: Vec<InboxPatchRead>,

    /// mark notifications as unread
    #[serde(default)]
    pub mark_unread: Vec<InboxPatchUnread>,

    /// add something to the thread as a notification
    #[serde(default)]
    pub add: Vec<InboxPatchAdd>,

    /// remove all old notifications before this timestamp
    pub prune_before: Option<Time>,
    // /// remove all old notifications from things with this id
    // pub prune: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum InboxPatchRead {
    Thread {
        room_id: RoomId,
        thread_id: ThreadId,
        read_at: Option<Time>,
    },
    Message {
        room_id: RoomId,
        thread_id: ThreadId,
        message_id: MessageId,
        read_at: Option<Time>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum InboxPatchUnread {
    Thread {
        room_id: RoomId,
        thread_id: ThreadId,
    },
    Message {
        room_id: RoomId,
        thread_id: ThreadId,
        message_id: MessageId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum InboxPatchAdd {
    Thread {
        room_id: RoomId,
        thread_id: ThreadId,
        /// defaults to now
        added_at: Option<Time>,
    },
    Message {
        room_id: RoomId,
        thread_id: ThreadId,
        message_id: MessageId,
        /// defaults to now
        added_at: Option<Time>,
    },
}

#[non_exhaustive] // remove for v1
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotificationReasonMessage {
    // /// this is a bookmark
    // Bookmark,
    /// this is a reminder
    Reminder,

    /// this message mentioned you
    MentionsUser,

    /// this message mentioned @room, @thread, or roles
    MentionsBulk,

    /// this message replied to one of your own messages
    Reply,
}

#[non_exhaustive] // remove for v1
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotificationReasonThread {
    /// this is a reminder
    Reminder,

    /// you are a thread member and there are new messages
    JoinedUnread,

    /// suggested thread you might like
    Suggestion,
}

/// Which notifications to include
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct InboxFilters(pub Vec<InboxFilter>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum InboxFilter {
    /// Get reminders.
    Reminder,

    /// Get user mentions.
    MentionsUser,

    /// Get "bulk" (@room, @thread) mentions.
    MentionsBulk,

    /// Get replies
    Reply,

    /// Get threads that the user is participating in.
    JoinedUnread,

    /// Get "interesting" threads.
    Suggestion,

    /// Include already read notifications.
    IncludeRead,

    /// Include muted threads and rooms.
    IncludeMuted,
    // probably not a good idea
    // /// Include ignored users.
    // IncludeIgnored,
}

impl Default for InboxFilters {
    fn default() -> Self {
        Self(vec![
            InboxFilter::Reminder,
            InboxFilter::MentionsUser,
            InboxFilter::MentionsBulk,
            InboxFilter::Reply,
            InboxFilter::JoinedUnread,
            InboxFilter::Suggestion,
        ])
    }
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
