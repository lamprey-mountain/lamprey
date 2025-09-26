use std::{sync::Arc, time::Instant};

use common::v1::types::{
    voice::{MediaKind, SfuPermissions, SignallingMessage, Speaking, VoiceState},
    ThreadId, UserId,
};
use str0m::{
    format::PayloadParams,
    media::{KeyframeRequestKind, MediaKind as MediaKindStr0m, MediaTime, Mid, Rid},
};

pub mod backend;
pub mod config;
pub mod peer;
pub mod sfu;
pub mod signalling;
pub mod state;
pub mod util;

/// an event emitted by the peer and handled by the sfu
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub enum PeerCommand {
    /// we got a signalling message from the user
    Signalling(SignallingMessage),

    /// user has a new voice state
    VoiceState(VoiceState),

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

    /// permissions
    Permissions(SfuPermissions),
}

#[derive(Debug, Clone)]
pub struct PeerPermissions {
    pub speak: bool,
    pub video: bool,
    pub priority: bool,
}

/// a peer event with user_id, so the sfu knows where the event came from
#[derive(Debug)]
pub struct PeerEventEnvelope {
    pub user_id: UserId,
    pub payload: PeerEvent,
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

/// errors that can be emitted from the sfu
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// no voice state exists for this user
    #[error("no voice state exists for this user")]
    NotConnected,

    /// the `Have` message is only sent by the server
    #[error("the `Have` message is only sent by the server")]
    HaveServerOnly,
}

impl From<SfuPermissions> for PeerPermissions {
    fn from(value: SfuPermissions) -> Self {
        Self {
            speak: value.speak,
            video: value.video,
            priority: value.priority,
        }
    }
}
