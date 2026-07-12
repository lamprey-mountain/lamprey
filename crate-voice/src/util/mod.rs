use std::collections::{HashMap, HashSet};

use crate::prelude::*;
use common::{
    v1::types::{
        util::Time,
        voice::{MediaKind, TrackKey, TrackLayer, VoiceState, internal::SfuPermissions},
    },
    v2::types::{ChannelId, SfuId},
};
use slotmap::{SlotMap, new_key_type};

pub mod permissions;
pub mod simulcast;
pub mod stun;

new_key_type! {
    /// slotmap key for a webrtc peer
    pub struct PeerSlot;

    /// slotmap key for a track
    ///
    /// mids are local to each peer, `TrackId`s are shared
    pub struct TrackSlot;

    pub struct SinkSlot;
    pub struct CallSlot;
}

// TODO: remove
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

pub struct Track {
    pub publisher: PeerSlot,
    pub kind: MediaKind,
    pub key: TrackKey,
    pub layers: Vec<TrackLayer>,

    /// the track state for the *publisher* of the track
    // NOTE: maybe i want to remove this from Track and make TrackState management part of Peer?
    pub state: TrackState,
}

pub struct Sink {
    pub subscriber: PeerSlot,
    pub source: TrackSlot,
    pub state: TrackState,
}

/// a voice state with extra info, for the server
pub struct SfuVoiceState {
    pub inner: VoiceState,
    pub permissions: SfuPermissions,
}

/// voice state for a cascade peer
// NOTE: may remove if this isnt useful
pub struct CascadeVoiceState {
    pub sfu_id: SfuId,
    pub channel_id: ChannelId,
    pub joined_at: Time,
}

impl Track {
    /// whether this track should always be subscribed
    // NOTE: im not sure if this is a good idea or not. this feels like it could retrospectively be a strange edge case.
    pub fn is_always_subscribed(&self) -> bool {
        self.kind == MediaKind::Audio && self.key == TrackKey::User
    }
}

// TODO: use this for routing media
#[derive(Default)]
pub struct Router {
    pub links: HashMap<TrackSlot, HashSet<SinkSlot>>,
    pub subscriptions: HashMap<(PeerSlot, TrackSlot), SinkSlot>,
}

impl Router {
    pub fn subscribe(
        &mut self,
        subscriber: PeerSlot,
        source: TrackSlot,
        sinks: &mut SlotMap<SinkSlot, Sink>,
    ) {
        if self.subscriptions.contains_key(&(subscriber, source)) {
            return;
        }

        let sink_id = sinks.insert(Sink {
            subscriber,
            source,
            state: TrackState::Pending,
        });

        self.links.entry(source).or_default().insert(sink_id);
        self.subscriptions.insert((subscriber, source), sink_id);
    }
}
