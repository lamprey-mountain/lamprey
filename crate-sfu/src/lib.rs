use std::{sync::Arc, time::Instant};

use common::v1::types::{
    voice::{MediaKind, SignallingMessage},
    ThreadId, UserId,
};
use serde::{Deserialize, Serialize};
use str0m::{
    format::PayloadParams,
    media::{KeyframeRequestKind, MediaKind as MediaKindStr0m, MediaTime, Mid, Rid},
};

pub mod config;
pub mod peer;
pub mod sfu;
pub mod util;

#[derive(Debug)]
pub struct PeerEventEnvelope {
    pub user_id: UserId,
    pub payload: PeerEvent,
}

#[derive(Debug)]
pub enum PeerEvent {
    Signalling(SignallingMessage),
    MediaAdded(TrackMetadataSfu),
    MediaData(MediaData),
    Dead,
    NeedsKeyframe {
        source_mid: Mid,
        source_peer: UserId,
        rid: Option<Rid>,
        for_peer: UserId,
        kind: KeyframeRequestKind,
    },
    Have {
        tracks: Vec<TrackMetadataServer>,
    },
    WantHave {
        user_ids: Vec<UserId>,
    },
    Speaking(Speaking),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Speaking {
    user_id: UserId,
    flags: SpeakingFlags,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpeakingWithoutUserId {
    flags: SpeakingFlags,
}

/// Flags for speaking
///
/// Audio = 1 << 0
/// Indicator = 1 << 1
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(transparent)]
pub struct SpeakingFlags(pub u8);

#[derive(Debug)]
pub enum PeerCommand {
    Signalling(SignallingMessage),
    MediaAdded(TrackMetadataSfu),
    MediaData(MediaData),
    Kill,
    GenerateKeyframe {
        mid: Mid,
        rid: Option<Rid>,
        kind: KeyframeRequestKind,
        for_peer: UserId,
    },
    Have {
        user_id: UserId,
        tracks: Vec<TrackMetadataServer>,
    },
    Speaking(Speaking),
}

#[derive(Debug, Clone)]
pub struct TrackMetadataServer {
    pub source_mid: Mid,
    pub kind: MediaKind,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct TrackMetadataSfu {
    pub source_mid: Mid,
    pub peer_id: UserId,
    pub thread_id: ThreadId,
    pub kind: MediaKindStr0m,
    pub key: String,
}

#[derive(Debug)]
pub struct TrackIn {
    pub kind: MediaKindStr0m,
    pub state: TrackState,
    pub thread_id: ThreadId,
    pub key: String,
}

#[derive(Debug)]
pub struct TrackOut {
    pub kind: MediaKindStr0m,
    pub state: TrackState,
    pub peer_id: UserId,
    pub source_mid: Mid,
    pub enabled: bool,
    pub thread_id: ThreadId,
    pub key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub struct MediaData {
    pub mid: Mid,
    pub peer_id: UserId,
    pub network_time: Instant,
    pub time: MediaTime,
    pub data: Arc<[u8]>,
    pub params: PayloadParams,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// no voice state exists for this user
    #[error("no voice state exists for this user")]
    NotConnected,
}
