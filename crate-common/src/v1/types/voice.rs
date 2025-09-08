// TODO: either copy thread/voice.rs to this or copy this to thread/voice.rs
// TODO: standardize terminology - everything is pretty loose right now

// current model:
// voice threads can have an associated call. calls have voicemembers. sfus
// exist in servers and regions

use std::ops::Deref;

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{util::Time, SessionId, UserId};

use super::ThreadId;

/// webrtc session description
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionDescription(pub String);

/// webrtc ice candidate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct IceCandidate(pub String);

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

/// represents a user that is connected to a voice thread (older docs call this a "voice connection")
///
/// connection limits:
/// - users can only have one active connection across all threads
/// - bots can connect to multiple threads with any connection strategy
/// - both users and bots can only have one connection per thread
// TODO: enforce the constraints listed above
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceState {
    /// the user this state belongs to
    pub user_id: UserId,

    /// the thread this user is connected to
    pub thread_id: ThreadId,

    /// the session that's being used to connect to this voice thread
    /// this is only be returned for the user this state belongs to
    pub session_id: Option<SessionId>,

    /// when this user joined the call
    pub joined_at: Time,

    /// whether this user is muted by a moderator
    pub mute: bool,

    /// whether this user is deafened by a moderator
    pub deaf: bool,
    // useful for showing stuff in ui without connecting
    // pub self_deaf: bool,
    // pub self_mute: bool,
    // pub self_video: bool,
    // pub self_stream: bool,

    // later
    // pub suppress: bool,
    // pub requested_to_speak_at: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateUpdate {
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackMetadata {
    /// unique identifier for this track. equivilant to transceiver.mid
    pub mid: String,

    /// whether this track is for audio or video
    pub kind: MediaKind,

    /// group tracks together into streams; identical to ssrc but easier to manage client side
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum SignallingMessage {
    /// a sdp offer
    Offer {
        sdp: SessionDescription,
        tracks: Vec<TrackMetadata>,
    },

    /// a sdp answer
    Answer { sdp: SessionDescription },

    /// an ice candidate
    Candidate { candidate: IceCandidate },

    /// sent by server only
    Have {
        thread_id: ThreadId,
        user_id: UserId,
        tracks: Vec<TrackMetadata>,
    },

    /// sent by server and client
    /// replaces the previous Want
    // should i default to sending everything? or require sending a Want to receive any data?
    // TODO: server sent `Want`s
    // TODO: client sent `Want`s
    Want { tracks: Vec<String> },

    /// sent by client.
    VoiceState { state: Option<VoiceStateUpdate> },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaKind {
    Video,
    Audio,
}

// /// config the local user sets on someone else
// struct VoiceConfig {
//     mute: bool,
//
//     /// between 0 and 1.5, defaults to 1
//     volume: f64,
// }

// ========== EVERYTHING BELOW IS INTERNAL FOR BACKEND/VOICE ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SfuCommand {
    /// proxied signalling message from a user
    Signalling {
        /// the user who sent this
        user_id: UserId,
        inner: SignallingMessage,
    },

    /// upsert voice state
    VoiceState {
        user_id: UserId,
        thread_id: ThreadId,
        state: Option<VoiceState>,
    },
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type")]
pub enum SfuEvent {
    VoiceDispatch {
        user_id: UserId,
        payload: SignallingMessage,
    },
    VoiceDispatchBroadcast {
        thread_id: ThreadId,
        payload: SignallingMessage,
    },
    VoiceState {
        user_id: UserId,
        old: Option<VoiceState>,
        state: Option<VoiceState>,
    },
}
