use crate::{PeerCommand, TrackMetadataServer, TrackMetadataSfu};
use common::v1::types::{voice::VoiceState, UserId};
use dashmap::DashMap;
use std::fmt::Debug;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub struct SfuState {
    pub peers: DashMap<UserId, UnboundedSender<PeerCommand>>,
    pub voice_states: DashMap<UserId, VoiceState>,
    pub tracks: Vec<TrackMetadataSfu>,
    pub tracks_by_user: DashMap<UserId, Vec<TrackMetadataServer>>,
}

impl SfuState {
    pub fn new() -> Self {
        Self {
            peers: DashMap::new(),
            voice_states: DashMap::new(),
            tracks: Vec::new(),
            tracks_by_user: DashMap::new(),
        }
    }

    pub fn remove_peer(&mut self, user_id: &UserId) -> Option<UnboundedSender<PeerCommand>> {
        self.voice_states.remove(user_id);
        self.tracks_by_user.remove(user_id);
        self.tracks.retain(|t| t.peer_id != *user_id);
        self.peers.remove(user_id).map(|(_, peer)| peer)
    }

    pub fn add_track(&mut self, track: TrackMetadataSfu) {
        if self
            .tracks
            .iter()
            .any(|t| t.source_mid == track.source_mid && t.peer_id == track.peer_id)
        {
            return;
        }
        self.tracks.push(track);
    }
}
