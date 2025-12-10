use std::ops::Deref;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{util::Time, Channel, SessionId, SfuId, UserId};

use super::ChannelId;

/// webrtc session description
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionDescription(pub String);

/// webrtc ice candidate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct IceCandidate(pub String);

/// a unique identifier for a media track (corresponds to a transceiver in webrtc, or a Mid in str0m)
///
/// media track ids are unique per peer connection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackId(pub String);

/// a unique identifier for a track layer (corresponds to a rid in webrtc)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct LayerId(pub String);

impl Deref for SessionDescription {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for IceCandidate {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for TrackId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// represents a user that is connected to a voice channel (older docs call this a "voice connection")
///
/// connection limits:
/// - users can only have one active connection across all channels
/// - bots can connect to multiple channels with any connection strategy
/// - both users and bots can only have one connection per channel
// TODO: enforce the constraints listed above
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceState {
    /// the user this state belongs to
    pub user_id: UserId,

    /// the channel this user is connected to
    pub channel_id: ChannelId,

    /// the session that's being used to connect to this voice channel
    /// this is only be returned for the user this state belongs to
    pub session_id: Option<SessionId>,

    /// when this user joined the call
    pub joined_at: Time,

    /// whether this user is muted by a moderator
    pub mute: bool,

    /// whether this user is deafened by a moderator
    pub deaf: bool,

    // useful for showing stuff in ui without connecting
    pub self_deaf: bool,
    pub self_mute: bool,
    pub self_video: bool,
    pub self_screen: bool,
    // these can come later, if needed at all
    // pub suppress: bool,
    // pub requested_to_speak_at: Option<Time>,
    // /// the thumbnail for the user's stream. should be updated periodically.
    // pub thumbnail: Option<MediaId>,
}

/// represents an update that a user would like to make to their voice state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateUpdate {
    pub channel_id: ChannelId,
    pub self_deaf: bool,
    pub self_mute: bool,
    pub self_video: bool,
    pub self_screen: bool,
    // /// the thumbnail for the user's stream. should be updated periodically.
    // pub thumbnail: Option<MediaId>,
}

/// metadata about a track
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackMetadata {
    /// unique identifier for this track. equivalent to transceiver.mid
    pub mid: TrackId,

    /// whether this track is for audio or video
    pub kind: MediaKind,

    /// group tracks together into streams; identical to ssrc but easier to manage client side
    ///
    /// currently there are two streams `user` and `screen` used by frontend
    pub key: String,
    // /// simulcasting layers, only applicable for video
    // #[serde(default, skip_serializing_if = "Vec::is_empty")]
    // pub layers: Vec<TrackLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackLayer {
    pub rid: LayerId,
    pub encoding: TrackEncoding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum TrackEncoding {
    /// source resolution
    Source,

    /// reduced thumbnail resolution
    Reduced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Subscription {
    pub mid: TrackId,

    /// the layers of the track to subscribe to
    ///
    /// - clients should only subscribe to one layer at a time
    /// - the server may subscribe to multiple depending on if multiple resolutions are requested
    /// - leave empty for audio tracks
    pub rid: Vec<LayerId>,
}

/// messages that either the sfu or client can send to each other
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum SignallingMessage {
    /// the allocated sfu is ready to accept voice payloads
    // NOTE: do i get rid of this and have VoiceState be the ready message? ie.
    // send a VoiceState once the voice server has been successfully allocated.
    // probably not tbh
    Ready {
        /// the id of the selected sfu. internal; for debugging.
        sfu_id: SfuId,
    },

    /// a sdp offer
    Offer {
        sdp: SessionDescription,
        tracks: Vec<TrackMetadata>,
    },

    /// a sdp answer
    Answer { sdp: SessionDescription },

    /// an ice candidate
    Candidate { candidate: IceCandidate },

    /// mapping of media ids to streams. sent by server only
    Have {
        channel_id: ChannelId,
        user_id: UserId,
        tracks: Vec<TrackMetadata>,
    },

    /// sent by server and client
    /// replaces the previous Want
    // should i default to sending everything? or require sending a Want to receive any data?
    // TODO: server sent `Want`s
    // TODO: client sent `Want`s
    Want { tracks: Vec<TrackId> },
    // Want { subscriptions: Vec<Subscription> },
    /// sent by client to update their voice state (including disconnecting)
    // TODO: move this to sync.rs
    VoiceState { state: Option<VoiceStateUpdate> },

    /// trigger a full reset; client should dispose current RTCPeerConnection and create a new one
    /// also useful to switch connection to another session
    Reconnect,
    // /// an error emitted by the sfu
    // Error {
    //     message: String,
    //     // code: VoiceErrorCode,
    // },
}

// #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub enum VoiceErrorCode {
//     Other,
// }

/// the kind of media this track is for
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaKind {
    Video,
    Audio,
}

/// Flags for speaking
///
/// Audio = 1 << 0
/// Indicator = 1 << 1
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(transparent)]
pub struct SpeakingFlags(pub u8);

impl SpeakingFlags {
    #[inline]
    pub fn has_audio(&self) -> bool {
        self.0 & 1 == 1
    }

    #[inline]
    pub fn has_indicator(&self) -> bool {
        self.0 & 2 == 2
    }
}

/// a message sent to the client to indicate that someone is speaking
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Speaking {
    pub user_id: UserId,
    // pub track_id: TrackId,
    pub flags: SpeakingFlags,
}

/// a message sent from the client to indicate that they're speaking
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpeakingWithoutUserId {
    pub flags: SpeakingFlags,
    // pub track_id: TrackId,
}

impl VoiceState {
    pub fn muted(&self) -> bool {
        self.mute || self.self_mute
    }

    pub fn deafened(&self) -> bool {
        self.deaf || self.self_deaf
    }
}

// ========== EVERYTHING BELOW IS INTERNAL FOR BACKEND/VOICE ==========

/// emitted by backend, handled by the sfu
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfuPermissions {
    /// corresponds to VoiceSpeak
    pub speak: bool,

    /// corresponds to VoiceVideo
    pub video: bool,

    /// corresponds to VoicePriority
    pub priority: bool,
}

/// channel config that the sfu needs to know about
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    use super::MediaKind;
    use str0m::media::MediaKind as MediaKindStr0m;

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
