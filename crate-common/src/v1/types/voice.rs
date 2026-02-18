use std::ops::Deref;

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    util::{some_option, Time},
    ConnectionId, MediaId, RoomId, SessionId, SfuId, UserId,
};

use super::ChannelId;

pub mod internal;

pub use internal::*;

/// webrtc session description
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionDescription(pub String);

/// webrtc ice candidate
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct IceCandidate(pub String);

/// a unique identifier for a media track (corresponds to a transceiver in webrtc, or a Mid in str0m)
///
/// media track ids are unique per peer connection
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackId(pub String);

/// a unique identifier for a track layer (corresponds to a rid in webrtc)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceState {
    /// the user this state belongs to
    pub user_id: UserId,

    /// the channel this user is connected to
    pub channel_id: ChannelId,

    /// the session that's being used to connect to this voice channel
    /// this is only be returned for the user this state belongs to
    pub session_id: Option<SessionId>,

    /// the connection id for this voice connection
    pub connection_id: Option<ConnectionId>,

    /// when this user joined the call
    pub joined_at: Time,

    /// whether this user is muted by a moderator
    pub mute: bool,

    /// whether this user is deafened by a moderator
    pub deaf: bool,

    /// whether this user has muted themselves
    pub self_mute: bool,

    /// whether this user has deafened themselves
    pub self_deaf: bool,

    /// whether this user has enabled their camera
    pub self_video: bool,

    /// populated if the user is sharing their screen
    pub screenshare: Option<VoiceStateScreenshare>,

    /// whether this user is suppressed, similar to a transient `mute: true`
    pub suppress: bool,

    /// when this user requested to speak
    pub requested_to_speak_at: Option<Time>,
}

/// info about a user's screen share
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateScreenshare {
    /// when this user started sharing their screen
    pub started_at: Time,

    /// the thumbnail for the user's stream. should be updated periodically.
    pub thumbnail: Option<MediaId>,
}

/// represents an update that a user would like to make to their voice state
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateUpdate {
    pub channel_id: ChannelId,
    pub self_deaf: bool,
    pub self_mute: bool,
    pub self_video: bool,
    pub screenshare: Option<VoiceStateStreamUpdate>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateStreamUpdate {
    /// the thumbnail for the user's stream. should be updated periodically.
    pub thumbnail: Option<MediaId>,
}

/// metadata about a track
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackMetadata {
    /// unique identifier for this track. equivalent to transceiver.mid
    pub mid: TrackId,

    /// whether this track is for audio or video
    pub kind: MediaKind,

    /// group tracks together into streams; identical to ssrc but easier to manage client side
    ///
    /// currently there are two streams `user` and `screen` used by frontend
    pub key: TrackKey,

    /// simulcasting layers, only applicable for video
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub layers: Vec<TrackLayer>,
}

/// which stream this track is associated with. generally there will be one video track and one audio track per stream.
// TODO: allow track keys not defined here?
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum TrackKey {
    User,
    Screen,
}

// TODO: specify bitrate, framerate, resolution?
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackLayer {
    pub rid: LayerId,
    pub encoding: TrackEncoding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum TrackEncoding {
    /// source resolution
    Source,

    /// reduced thumbnail resolution
    Reduced,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Subscription {
    pub mid: TrackId,

    /// the layers of the track to subscribe to
    ///
    /// - clients should only subscribe to one layer at a time, but multiple can be subscribed if needed
    /// - the server may subscribe to multiple depending on if multiple resolutions are requested
    /// - leave empty for audio tracks
    pub rid: Vec<LayerId>,
}

/// messages that either the sfu or client can send to each other
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
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
    // NOTE: currently unused by both client and server
    Candidate { candidate: IceCandidate },

    /// mapping of media ids to streams. sent by server only
    Have {
        channel_id: ChannelId,
        user_id: UserId,
        tracks: Vec<TrackMetadata>,
    },

    /// request additional tracks
    ///
    /// - all audio from track key `user` is sent by default
    /// - all video and audio from other sources require a Want
    /// - sent by server and client
    /// - replaces the previous Want
    // TODO: implement server sent `Want`s
    // TODO: implement client sent `Want`s
    Want { subscriptions: Vec<Subscription> },

    /// sent by client to update their voice state (including disconnecting)
    // TODO: merge this with MessageSync::VoiceState in sync.rs
    VoiceState { state: Option<VoiceStateUpdate> },

    /// trigger a full reset; client should dispose current RTCPeerConnection and create a new one
    /// also useful to switch connection to another session
    // NOTE: this is hacky and ideally could be replaced with better peer connection and transceiver management altogether
    Reconnect,

    /// an error emitted by the sfu
    Error {
        /// human readable error message
        message: String,

        /// what exactly went wrong
        code: VoiceErrorCode,
    },
}

// this may be upgraded to a full error struct later, instead of only code
#[derive(Debug, Clone, Error)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum VoiceErrorCode {
    /// unknown track (webrtc mid)
    #[error("unknown mid")]
    UnknownTrack,

    /// unknown layer (webrtc rid)
    #[error("unknown rid")]
    UnknownLayer,

    /// unknown/other error
    #[error("unknown/other error")]
    Other,
}

/// the kind of media this track is for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaKind {
    Video,
    Audio,
}

/// Flags for speaking
///
/// Audio = 1 << 0
/// Indicator = 1 << 1
/// Priority = 1 << 2
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
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

    #[inline]
    pub fn has_priority(&self) -> bool {
        self.0 & 4 == 4
    }
}

/// a message sent to the client to indicate that someone is speaking
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Speaking {
    pub user_id: UserId,
    // pub track_id: TrackId,
    pub flags: SpeakingFlags,
}

/// a message sent from the client to indicate that they're speaking
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpeakingWithoutUserId {
    pub flags: SpeakingFlags,
    // pub track_id: TrackId,
}

impl VoiceState {
    pub fn muted(&self) -> bool {
        self.mute || self.self_mute || self.suppress
    }

    pub fn deafened(&self) -> bool {
        self.deaf || self.self_deaf
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateMove {
    pub target_id: ChannelId,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateMoveBulk {
    /// set to None to move everyone
    pub user_ids: Option<Vec<UserId>>,

    /// target channel id
    pub channel_id: ChannelId,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStatePatch {
    /// allow this user to speak in the current channel
    ///
    /// requires VoiceMute permission
    pub suppress: Option<bool>,

    /// same as room member deaf
    pub deaf: Option<bool>,

    /// same as room member mute
    pub mute: Option<bool>,

    /// where to move this participant. you can only move participants to the channels in the same room.
    pub channel_id: Option<ChannelId>,

    /// when this user requested to speak
    ///
    /// - users can only set this for themselves
    /// - this can only be set to the current time
    /// - you must have VoiceRequest to set this
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub requested_to_speak_at: Option<Option<Time>>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CallCreate {
    /// channel to create a call in
    ///
    /// must be a Broadcast channel
    pub channel_id: ChannelId,

    /// call topic
    ///
    /// must have VoiceMute permission in target channel to set
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 512)))]
    pub topic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Call {
    pub room_id: Option<RoomId>,
    pub channel_id: ChannelId,
    pub topic: Option<String>,
    pub created_at: Time,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CallPatch {
    /// the current call topic
    ///
    /// only unsuppressed users can change the call topic
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 512)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub topic: Option<Option<String>>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct CallDeleteParams {
    /// if people are still connected to this channel, try to forcibly disconnect them
    ///
    /// requires VoiceDisconnect permission
    #[cfg_attr(feature = "serde", serde(default))]
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RingEligibility {
    /// whether ring endpoints can be used
    ///
    /// true in dms and gdms, false otherwise
    pub ringable: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RingStart {
    pub user_ids: Vec<UserId>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RingStop {
    pub user_ids: Vec<UserId>,
}
