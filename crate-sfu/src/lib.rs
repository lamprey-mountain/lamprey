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

/// an event emitted by the peer and handled by the sfu
#[derive(Debug)]
pub enum PeerEvent {
    /// send a signalling message to the peer user
    Signalling(SignallingMessage),

    /// we have a new inbound media track
    MediaAdded(TrackMetadataSfu),

    /// we received media data from our peer
    MediaData(MediaData),

    /// we are permanently dead and won't send/recv data anymore
    Dead,

    /// our peer wants a keyframe for this media
    NeedsKeyframe {
        source_mid: Mid,
        source_peer: UserId,
        rid: Option<Rid>,
        for_peer: UserId,
        kind: KeyframeRequestKind,
    },

    /// we have these tracks
    Have { tracks: Vec<TrackMetadataServer> },

    /// we want a Have message for these user ids
    WantHave { user_ids: Vec<UserId> },

    /// the peer is speaking
    Speaking(Speaking),
}

/// an command emitted by the sfu and handled by the peer
#[derive(Debug)]
pub enum PeerCommand {
    /// we got a signalling message from the user
    Signalling(SignallingMessage),

    /// a remote peer created a new track
    MediaAdded(TrackMetadataSfu),

    /// a remote peer sent some media data
    MediaData(MediaData),

    /// tell the the peer to stop (and emit a Dead event)
    Kill,

    /// a remote peer wants a keyframe for this media
    GenerateKeyframe {
        mid: Mid,
        rid: Option<Rid>,
        kind: KeyframeRequestKind,
        for_peer: UserId,
    },

    /// a remote peer has these tracks
    Have {
        user_id: UserId,
        tracks: Vec<TrackMetadataServer>,
    },

    /// a remote peer is speaking
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
