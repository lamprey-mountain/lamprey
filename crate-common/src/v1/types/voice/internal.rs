use bitflags::bitflags;

use lamprey_macros::record;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::v1::types::{ConnectionId, SessionId, UserId, misc::Time, voice::VoiceState};

/// smaller voice state for sfus
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SfuVoiceState {
    pub user_id: UserId,
    pub session_id: SessionId,
    pub connection_id: ConnectionId,
    pub joined_at: Time,
    pub flags: SfuVoiceFlags,
}

bitflags! {
    /// permissions for an sfu peer
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct SfuVoiceFlags: u8 {
        /// whether the user is mute
        ///
        /// doesn't have the `VoiceSpeak` permission, is `mute`d, is `self_mute`d, or is `suppress`ed
        const Mute = 1 << 0;

        /// whether the user can send video
        ///
        /// has the `VoiceVideo` permission and isn't `suppress`ed
        const Video = 1 << 1;

        /// whether the user can use priority speaker
        ///
        /// has the `VoicePriority` permission and isn't mute
        const Priority = 1 << 2;

        /// whether the user is deaf
        ///
        /// is `mute`d or is `self_mute`d
        const Deaf = 1 << 3;
    }
}

/// configuration for a voice call
#[record]
pub struct VoiceConfig {
    /// the name of a voice channel, for debugging
    pub name: String,

    /// maximum bitrate for audio tracks
    pub bitrate: u64,
    // TODO: video resolution
}

/// errors that occur when converting from an api voice state to an sfu voice state
#[derive(Debug, Clone, Error)]
pub enum SfuVoiceStateConversionError {
    /// the voice state is missing a session id
    #[error("missing session id")]
    MissingSessionId,

    /// the voice state is missing a connection id
    #[error("missing connection id")]
    MissingConnectionId,
}

impl SfuVoiceState {
    #[inline]
    pub fn can_speak(&self) -> bool {
        !self.flags.contains(SfuVoiceFlags::Mute)
    }

    #[inline]
    pub fn can_screenshare(&self) -> bool {
        self.flags.contains(SfuVoiceFlags::Video)
    }

    #[inline]
    pub fn can_use_priority(&self) -> bool {
        self.flags.contains(SfuVoiceFlags::Priority)
    }

    pub fn from_api_state(
        vs: VoiceState,
        priority: bool,
    ) -> Result<Self, SfuVoiceStateConversionError> {
        let mut flags = SfuVoiceFlags::empty();
        if vs.muted() {
            flags |= SfuVoiceFlags::Mute;
        }
        if vs.self_video && !vs.suppress {
            flags |= SfuVoiceFlags::Video;
        }
        if priority && !vs.muted() {
            flags |= SfuVoiceFlags::Priority;
        }
        if vs.deafened() {
            flags |= SfuVoiceFlags::Deaf;
        }

        Ok(Self {
            user_id: vs.user_id,
            session_id: vs
                .session_id
                .ok_or(SfuVoiceStateConversionError::MissingSessionId)?,
            connection_id: vs
                .connection_id
                .ok_or(SfuVoiceStateConversionError::MissingConnectionId)?,
            joined_at: vs.joined_at,
            flags,
        })
    }
}
