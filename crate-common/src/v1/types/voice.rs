// TODO: either copy thread/voice.rs to this or copy this to thread/voice.rs
// TODO: standardize terminology - everything is pretty loose right now

// current model:
// voice threads can have an associated call. calls have voicemembers. sfus
// exist in servers and regions

use std::ops::Deref;

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{util::Time, UserId};

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

    // /// the session that's being used to connect to this voice thread
    // /// this will only be returned for the user this state belongs to
    // pub session_id: Option<SessionId>,
    /// when this user joined the call
    pub joined_at: Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStateUpdate {
    pub thread_id: ThreadId,
}

// if i move stuff perms into voice member/states
// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct VoiceMemberV2 {
//     pub user_id: UserId,
//     pub room_id: RoomId,
//     pub thread_id: ThreadId,
//     pub call_id: CallId,
//     pub session_id: (),

//     pub joined_at: Time,
//     pub deaf: bool,
//     pub mute: bool,
//     pub self_deaf: bool,
//     pub self_mute: bool,

//     // pub self_video: bool,
//     // pub self_stream: bool,
//     pub video: Vec<()>, // includes user and display media

//     pub suppress: bool,
//     pub requested_to_speak_at: Option<Time>,
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TrackMetadata {
    pub mid: String,
    pub kind: MediaKindSerde,

    // group tracks together into streams; identical to ssrc but easier to manage client side
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
    Candidate {
        candidate: IceCandidate,
        // not supported by str0m or not needed at all?
        // sdp_mid: Mid,
        // sdp_mline_index: u16,
    },

    // sent by server only
    Have {
        thread_id: ThreadId,
        user_id: UserId,
        tracks: Vec<TrackMetadata>,
    },

    /// sent by server and client
    Want {
        // tracks: Vec<Mid>,
        tracks: Vec<String>,
    },

    /// sent by client.
    VoiceState { state: Option<VoiceStateUpdate> },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MediaKindSerde {
    Video,
    Audio,
}
