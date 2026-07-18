use common::v1::types::voice::{MediaKind, TrackKey, TrackMetadata2};

use crate::prelude::*;

/// the current state of a webrtc track
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackState {
    /// track exists but needs to be created
    Pending,

    /// we have it in our local sdp, needs to be sent to peer
    Negotiating(SMid),

    /// data can be sent through this track
    Open(SMid),

    /// this track is going to be closed
    Closing(SMid),
    // Closed,
}

impl TrackState {
    pub fn mid(&self) -> Option<SMid> {
        match self {
            TrackState::Pending => None,
            TrackState::Negotiating(mid) => Some(*mid),
            TrackState::Open(mid) => Some(*mid),
            TrackState::Closing(mid) => Some(*mid),
        }
    }
}

/// info about a track that the sfu is receiving
// formerly called Track
pub struct Inbound {
    pub publisher: PeerSlot,
    pub metadata: TrackMetadata2,
    pub state: TrackState,
}

/// info about a track that the sfu is forwarding to a peer
// formerly called Sink
pub struct Outbound {
    pub subscriber: PeerSlot,
    pub source: TrackSlot,
    pub state: TrackState,
}

impl Inbound {
    /// whether this track should always be subscribed
    // NOTE: im not sure if this is a good idea or not. this feels like it could retrospectively be a strange edge case.
    pub fn is_implicit(&self) -> bool {
        self.metadata.kind == MediaKind::Audio && self.metadata.key == TrackKey::User
    }

    /// returns the metadata for this track
    #[inline]
    pub fn metadata(&self) -> &TrackMetadata2 {
        // TODO: have actual track ids instead of keying by (mid, user_id) (and sometimes (kind, key)) (?)
        &self.metadata
    }

    /// get the kind of this media track
    #[inline]
    pub fn kind(&self) -> MediaKind {
        self.metadata.kind
    }
}
