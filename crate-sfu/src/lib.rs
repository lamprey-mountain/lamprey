// do i have all signalling go through the main events websocket, or only
// do sdp/ice/connection and do signalling directly against the sfu with
// datachannels? i feel like the second could be nicer but harder.

use std::{sync::Arc, time::Instant};

use common::v1::types::{
    voice::{IceCandidate, SessionDescription, VoiceState, VoiceStateUpdate},
    UserId,
};
use serde::{Deserialize, Serialize};
use str0m::{
    format::PayloadParams,
    media::{MediaKind, MediaTime, Mid},
    Candidate,
};

pub mod peer;
pub mod sfu;
pub mod util;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignallingMessage {
    Offer {
        sdp: SessionDescription,
        tracks: Vec<TrackMetadata>,
    },

    Answer {
        sdp: SessionDescription,
    },

    Candidate {
        candidate: IceCandidate,
        // not supported by str0m or not needed at all?
        // sdp_mid: Mid,
        // sdp_mline_index: u16,
    },

    // sent by server only
    Have {
        user_id: UserId,
        tracks: Vec<TrackMetadata>,
    },

    // sent by server and users
    Want {
        tracks: Vec<Mid>,
    },

    // sent by client
    VoiceState {
        state: Option<VoiceStateUpdate>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SfuCommand {
    /// the user who sent this, or None if this is from the server
    pub user_id: Option<UserId>,

    #[serde(flatten)]
    pub inner: SignallingMessage,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type")]
pub enum SfuEvent {
    VoiceDispatch {
        user_id: UserId,
        payload: SignallingMessage,
    },
    VoiceState {
        user_id: UserId,
        old: Option<VoiceState>,
        state: Option<VoiceState>,
    },
}

#[derive(Debug)]
pub struct PeerEventEnvelope {
    pub user_id: UserId,
    pub payload: PeerEvent,
}

#[derive(Debug)]
pub enum PeerEvent {
    Signalling(SignallingMessage),
    MediaAdded(SfuTrack),
    MediaData(MediaData),
    Dead,
    Init,
}

#[derive(Debug)]
pub enum PeerCommand {
    Signalling(SignallingMessage),
    MediaAdded(SfuTrack),
    MediaData(MediaData),
    // RemotePublish {
    //     user_id: UserId,
    //     mid: Mid,
    //     key: String,
    // },
    Kill,
}

#[derive(Debug, Clone)]
pub struct MediaData {
    pub mid: Mid,
    pub peer_id: UserId,
    pub network_time: Instant,
    pub time: MediaTime,
    pub data: Arc<[u8]>,
    pub params: PayloadParams,
}

#[derive(Debug, Clone)]
pub struct SfuTrack {
    pub mid: Mid,
    pub peer_id: UserId,
    pub kind: MediaKind,
    // TODO: replace with ssrc
    pub key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackState {
    Pending,
    Negotiating(Mid),
    Open(Mid),
}

impl TrackState {
    pub fn mid(&self) -> Option<Mid> {
        match self {
            TrackState::Pending => None,
            TrackState::Negotiating(mid) => Some(*mid),
            TrackState::Open(mid) => Some(*mid),
        }
    }
}

#[derive(Debug)]
pub struct TrackIn {
    pub kind: MediaKind,
    pub state: TrackState,
}

#[derive(Debug)]
pub struct TrackOut {
    pub kind: MediaKind,
    pub state: TrackState,
    pub peer_id: UserId,
    pub source_mid: Mid,
    pub enabled: bool,
    pub needs_keyframe: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// no voice state exists for this user
    #[error("no voice state exists for this user")]
    NotConnected,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MediaKindSerde {
    Video,
    Audio,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub mid: Mid,
    pub kind: MediaKindSerde,

    // group tracks together into streams; identical to ssrc but easier to manage client side
    pub key: String,
}
