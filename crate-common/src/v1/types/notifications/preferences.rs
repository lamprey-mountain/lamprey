//! user notification preference

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::util::Time;

/// notification config for a user (works globally)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotifsGlobal {
    pub mute: Option<Mute>,
    pub messages: NotifsMessages,
    pub threads: NotifsThreads,
    pub reactions: NotifsReactions,
}

/// notification config for a room
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotifsRoom {
    pub mute: Option<Mute>,

    /// how to handle new messages
    pub messages: Option<NotifsMessages>,

    /// how to handle new threads
    pub threads: Option<NotifsThreads>,

    /// whether to receive @everyone and @here mentions
    pub mention_everyone: bool,

    /// whether to receive all @role mentions
    pub mention_roles: bool,
}

/// notification config for a channel
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotifsChannel {
    pub mute: Option<Mute>,

    /// message notif config
    ///
    /// None means inherit from category/room
    pub messages: Option<NotifsMessages>,

    /// can't be set on voice and thread channels
    ///
    /// None means inherit from category/room
    pub threads: Option<NotifsThreads>,
}

/// how to handle new messages
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotifsMessages {
    /// notify on every message
    Everything,

    /// notify on mentions; add all new messages to inbox
    Watching,

    /// notify on mentions
    #[default]
    Mentions,

    /// don't receive any notifications for messages
    // how does this compare with Mute? maybe make mute *completely* hide
    // everything (including red mention circles), while this just doesnt notify
    // you
    Nothing,
}

/// how to handle new threads
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotifsThreads {
    /// notify whenever a new thread is created
    Notify,

    /// add all new threads to your inbox
    #[default]
    Inbox,

    /// ignore new threads
    Nothing,
}

// only affects private rooms
// TODO: implement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotifsVoice {
    /// when someone starts streaming
    Streams,

    /// when anyone connects
    Voice,

    /// never send a notification for voice/broadcast channels
    Nothing,
}

/// what notifications to send for reactions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotifsReactions {
    /// notify for all reactions
    Always,

    /// reactions in direct messages and private rooms only
    Restricted,

    /// reactions in direct messages only
    Dms,

    /// never send a notification for reactions
    Nothing,
}

/// how long to mute notifications for
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Mute {
    /// how long to mute for, or forever if None
    pub expires_at: Option<Time>,
}

impl Default for NotifsGlobal {
    fn default() -> Self {
        NotifsGlobal {
            mute: None,
            messages: NotifsMessages::Mentions,
            threads: NotifsThreads::Inbox,
            reactions: NotifsReactions::Restricted,
        }
    }
}
