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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceState {
    pub user_id: UserId,
    // pub room_id: RoomId,
    pub thread_id: ThreadId,
    // pub session_id: (),
    /// when this person joined the call
    pub joined_at: Time,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceStatePatch {
    pub thread_id: Option<ThreadId>,
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
