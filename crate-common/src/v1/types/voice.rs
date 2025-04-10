// TODO: either copy thread/voice.rs to this or copy this to thread/voice.rs

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{util::Time, UserId};

/// webrtc session description
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SessionDescription(pub String);

/// webrtc ice candidate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct IceCandidate(pub String);

// /// a call in progress
// pub struct Call {
//     // is this the same as thread_id?
//     pub id: CallId,
//     pub thread_id: ThreadId,
//     pub participant_count: u64,
//     // in ms; is current duration
//     // when Archived, is set to id - state_updated_at
//     // prevent state to Active? set duration once or every time state->Archived?
//     pub duration: u64,
// }

/// the state of a call participant
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceParticipant {
    // pub call_id: CallId,
    pub user_id: UserId,
    pub joined_at: Time,
    pub stream_started_at: Option<Time>,
    // pub is_muted: bool, // has_voice
    pub is_deafened: bool,
    pub has_voice: bool,
    pub has_camera: bool,
    // pub volume: Option<f64>,
}
