//! user notification preference

use lamprey_macros::record;

use crate::v1::types::util::Time;

/// notification config for a user (works globally)
#[record]
#[derive(Default)]
pub struct NotifsGlobal {
    pub mute: Option<Mute>,
    pub messages: NotifsMessages,
    pub replies: NotifsReplies,
    pub threads: NotifsThreads,
    pub reactions: NotifsReactions,
    pub tts: NotifsTts,
}

/// notification config for a room
#[record]
#[derive(Default)]
pub struct NotifsRoom {
    // TODO: add skip_serializing_if (will this break frontend?)
    // #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<Mute>,

    /// how to handle new messages
    pub messages: Option<NotifsMessages>,

    /// how to handle new replies
    pub replies: Option<NotifsReplies>,

    /// how to handle new threads
    pub threads: Option<NotifsThreads>,

    /// whether to receive @everyone and @here mentions
    pub mention_everyone: bool,

    /// whether to receive all @role mentions
    pub mention_roles: bool,
}

/// notification config for a channel
#[record]
#[derive(Default)]
pub struct NotifsChannel {
    pub mute: Option<Mute>,

    /// message notif config
    ///
    /// None means inherit from category/room
    pub messages: Option<NotifsMessages>,

    /// how to handle new replies
    pub replies: Option<NotifsReplies>,

    /// can't be set on voice and thread channels
    ///
    /// None means inherit from category/room
    pub threads: Option<NotifsThreads>,
}

/// how to handle new messages
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum NotifsMessages {
    /// notify on every message
    Everything,

    /// notify on mentions; add all new messages to inbox
    Watching,

    /// notify on mentions
    #[default]
    Mentions,

    /// don't receive any notifications for messages
    ///
    /// while muting completely disables notifications for something, this will
    /// show unread indicator and mention count in ui
    Nothing,
}

/// how to handle new replies
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum NotifsReplies {
    /// always notify for replies
    Notify,

    /// add all new replies to inbox
    #[default]
    Watching,

    /// don't treat replies specially
    Nothing,
}

/// how to handle new threads
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum NotifsThreads {
    /// notify whenever a new thread is created
    Notify,

    /// add all new threads to your inbox
    #[default]
    Inbox,

    /// ignore new threads
    Nothing,
}

/// what notifications to send for reactions
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum NotifsReactions {
    /// notify for all reactions
    Always,

    /// reactions in direct messages and private rooms only
    Restricted,

    /// reactions in direct messages only
    #[default]
    Dms,

    /// never send a notification for reactions
    Nothing,
}

/// when to send text to speech notifications
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum NotifsTts {
    /// read all messages that created a notification
    Always,

    /// read only mentions
    Mentions,

    /// never send tts notifications
    #[default]
    Nothing,
}

/// how long to mute notifications for
#[record]
#[derive(Default, PartialEq, Eq)]
pub struct Mute {
    /// how long to mute for, or forever if None
    pub expires_at: Option<Time>,

    /// the selected duration in seconds in the ui
    pub duration: Option<u64>,
}

// TODO: implement notification config for voice, documents, calendar events, redexes
// mention location: message, document?

/// when to send notifications for voice channels
///
/// only affects private rooms
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum NotifsVoice {
    /// when someone starts streaming
    Streams,

    /// when anyone connects
    Voice,

    /// never send a notification for voice/broadcast channels
    #[default]
    Nothing,
}
