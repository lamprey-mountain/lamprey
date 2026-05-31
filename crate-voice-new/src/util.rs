use common::v1::types::voice::{MediaKind, TrackKey};
use slotmap::new_key_type;
use smallvec::SmallVec;
use str0m::media::Mid as SMid;

pub mod signalling;

// TODO: doc comments
new_key_type! {
    pub struct PeerId;
    pub struct TrackId;
}

/// the current state of a webrtc track
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackState {
    /// track exists but needs to be created
    Pending,

    /// we have it in our local sdp, needs to be sent to peer
    Negotiating(SMid),

    /// data can be sent through this track
    Open(SMid),
}

impl TrackState {
    pub fn mid(&self) -> Option<SMid> {
        match self {
            TrackState::Pending => None,
            TrackState::Negotiating(mid) => Some(*mid),
            TrackState::Open(mid) => Some(*mid),
        }
    }
}

/// what the sfu knows about a track
pub struct TrackSfu {
    /// who is publishing this track
    pub publisher: PeerId,

    /// subscriptions to this track
    pub subscribers: SmallVec<[Subscriber; 8]>,

    // basic metadata
    pub kind: MediaKind,
    pub key: TrackKey,

    /// Track state (Pending, Negotiating, Open)
    pub state: TrackState,
}

/// a webrtc peer's request for a track
#[derive(Clone)]
pub struct Subscriber {
    /// the peer who is subscribed
    pub peer_id: PeerId,

    // // TODO: simulcast layers
    // pub rid: SmallVec<[Rid; 1]>,
    /// send media to this mid on the *receiver*
    pub sink_mid: SMid,
}
