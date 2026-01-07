#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::v1::types::{
    voice::{SignallingMessage, VoiceState},
    Channel, ChannelId, SfuId, UserId,
};

/// emitted by backend, handled by the sfu
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SfuCommand {
    Ready {
        sfu_id: SfuId,
    },

    /// proxied signalling message from a user
    Signalling {
        /// the user who sent this
        user_id: UserId,
        inner: SignallingMessage,
    },

    /// upsert voice state
    VoiceState {
        user_id: UserId,
        state: Option<VoiceState>,
        permissions: SfuPermissions,
    },

    /// upsert channel
    Channel {
        channel: SfuChannel,
    },
}

/// emitted by the sfu, handled by backend
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[serde(tag = "type")]
pub enum SfuEvent {
    /// send this message to this user
    VoiceDispatch {
        user_id: UserId,
        payload: SignallingMessage,
    },

    /// upsert voice state
    VoiceState {
        user_id: UserId,
        old: Option<VoiceState>,
        state: Option<VoiceState>,
    },
}

/// permissions that the sfu needs to know about
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SfuPermissions {
    /// corresponds to VoiceSpeak
    pub speak: bool,

    /// corresponds to VoiceVideo
    pub video: bool,

    /// corresponds to VoicePriority
    pub priority: bool,
}

/// channel config that the sfu needs to know about
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SfuChannel {
    pub id: ChannelId,
    pub name: String,
    pub bitrate: Option<u64>,
    pub user_limit: Option<u64>,
}

impl From<Channel> for SfuChannel {
    fn from(value: Channel) -> Self {
        Self {
            id: value.id,
            name: value.name,
            bitrate: value.bitrate,
            user_limit: value.user_limit,
        }
    }
}

#[cfg(feature = "str0m")]
mod str0m {
    use str0m::media::MediaKind as MediaKindStr0m;

    use crate::v1::types::voice::MediaKind;

    impl From<MediaKind> for MediaKindStr0m {
        fn from(value: MediaKind) -> Self {
            match value {
                MediaKind::Video => MediaKindStr0m::Video,
                MediaKind::Audio => MediaKindStr0m::Audio,
            }
        }
    }

    impl From<MediaKindStr0m> for MediaKind {
        fn from(value: MediaKindStr0m) -> Self {
            match value {
                MediaKindStr0m::Video => MediaKind::Video,
                MediaKindStr0m::Audio => MediaKind::Audio,
            }
        }
    }
}
