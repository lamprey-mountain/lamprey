#![allow(unused)]

use crate::{MessageId, MessageVerId, RoomId, ThreadId};

// a bunch of random ideas from past things
// TODO: pare and reduce these down

enum InboxFilter {
    /// The default filter: MentionsUser | MentionsBulk | ThreadsParticipating | ThreadsInteresting
    Default,

    /// Get user mentions.
    MentionsUser,

    /// Get "bulk" (@room, @thread) mentions.
    MentionsBulk,

    /// Get threads that the user is participating in.
    ThreadsParticipating,

    /// Get "interesting" threads.
    ThreadsInteresting,

    /// Include read threads.
    IncludeRead,

    /// Include read threads.
    IncludeIgnored,
}

struct Notification {
    pub room_id: RoomId,
    pub thread_id: ThreadId,
    pub message_id: MessageId,
    pub message_version_id: MessageVerId,
    pub read: bool,
}

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

enum Action {
    None,
    Inbox,
    Notify,
}

struct RoomConfig {
    new_thread: Option<Action>,
    new_message: Option<Action>,
    // new_message: Option<Action>,
}

enum NotificationType {
    /// when the thread is updated (name, description)
    ThreadUpdate,
    
    /// when the thread state is updated (archive, pin, unpin)
    ThreadStatus,

    /// message that mentions you
    MessageMention,

    /// message that replies to one of your messages
    MessageReply,

    /// message in a thread you're watching
    MessageWatching,
    
    /// message in a dm
    MessageDm,
}

enum NotificationAction {
    /// add to 
    Inbox,
    Notify,
}

struct NotificationConfig {
    config: Vec<(NotificationType, NotificationAction)>,
}

fn default_notification_config() -> NotificationConfig {
    NotificationConfig {
        config: vec![
            (NotificationType::MessageMention, NotificationAction::Notify),
            (NotificationType::MessageReply, NotificationAction::Inbox),
            (NotificationType::MessageWatching, NotificationAction::Inbox),
            (NotificationType::MessageDm, NotificationAction::Notify),
            (NotificationType::ThreadStatus, NotificationAction::Inbox),
            (NotificationType::ThreadUpdate, NotificationAction::Inbox),
        ]
    }
}
