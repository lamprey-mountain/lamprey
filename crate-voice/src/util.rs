use std::collections::{HashMap, HashSet};

use crate::prelude::*;
use common::{
    v1::types::{
        util::Time,
        voice::{
            internal::SfuPermissions, MediaKind, Subscription, TrackKey, TrackLayer, VoiceState,
        },
    },
    v2::types::{ChannelId, SfuId},
};
use slotmap::new_key_type;

pub mod permissions;
pub mod signalling;
pub mod simulcast;

new_key_type! {
    /// slotmap key for a webrtc peer
    pub struct PeerId;

    /// slotmap key for a track
    ///
    /// mids are local to each peer, `TrackId`s are shared
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
    // /// this track is going to be closed
    // Closing(SMid),
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

pub struct Track {
    pub publisher: PeerId,
    pub kind: MediaKind,
    pub key: TrackKey,
    pub layers: Vec<TrackLayer>,

    /// the track state for the *publisher* of the track
    // NOTE: maybe i want to remove this from Track and make TrackState management part of Peer?
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

/// what the peer is subscribed to
#[derive(Debug, Default)]
pub struct Subscriptions {
    /// list of this peer's subscriptions
    pub subs: Vec<Subscription>,

    /// which tracks are we currently subscribed to
    pub tracks: HashSet<TrackId>,

    /// if true, try to create missing tracks
    pub dirty: bool,
}

impl Subscriptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, subs: Vec<Subscription>, tracks: HashSet<TrackId>) {
        if self.tracks != tracks {
            self.dirty = true;
        }
        self.subs = subs;
        self.tracks = tracks;
    }

    // /// remove all subscriptions to a user's stream
    // ///
    // /// for when a peer disconnects
    // pub fn remove_user(&mut self, _user_id: UserId) {
    //     // TODO: Implement track lookup by user_id to filter `self.tracks`
    //     todo!()
    // }
}

// /// what the peer is subscribed to
// #[derive(Debug, Default)]
// pub struct Subscriptions {
//     // subs: Vec<Subscription>,
//     tracks: HashMap<TrackId, TrackState>,
// }

// impl Subscriptions {
//     pub fn new() -> Self {
//         Self::default()
//     }

//     /// subscribe to a track
//     pub fn add_track(&mut self, track_id: TrackId) {
//         self.tracks.insert(track_id, TrackState::Pending);
//     }

//     // TODO: remove_track
// }

// impl crate::peer::Peer {
//     fn arst(&mut self) {
//         let mut changes = self.rtc.sdp_api();
//         for (track_id, state) in self.subscriptions.tracks.iter_mut() {
//             if state == TrackState::Pending {
//                 let mid = changes.add_media(kind, dir, stream_id, track_id, simulcast);
//                 *state = TrackState::Negotiating(mid);
//             }
//         }
//         changes.apply();
//     }
// }
