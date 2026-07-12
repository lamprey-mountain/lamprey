use crate::prelude::SMid;
use crate::util::TrackSlot;
use std::collections::HashMap;

/// mappings between ids local to this peer and shared sfu identifiers
///
/// eg. `Mid`s to `TrackSlot`s
#[derive(Debug, Clone, Default)]
pub struct Mapping {
    track_to_mid: HashMap<TrackSlot, SMid>,
    mid_to_track: HashMap<SMid, TrackSlot>,
}

impl Mapping {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, mid: SMid, track: TrackSlot) {
        self.mid_to_track.insert(mid, track);
        self.track_to_mid.insert(track, mid);
    }

    pub fn remove_by_mid(&mut self, mid: SMid) {
        if let Some(track) = self.mid_to_track.remove(&mid) {
            self.track_to_mid.remove(&track);
        }
    }

    pub fn remove_by_track(&mut self, track: TrackSlot) {
        if let Some(mid) = self.track_to_mid.remove(&track) {
            self.mid_to_track.remove(&mid);
        }
    }

    pub fn lookup_mid(&self, track: TrackSlot) -> Option<SMid> {
        self.track_to_mid.get(&track).copied()
    }

    pub fn lookup_track(&self, mid: SMid) -> Option<TrackSlot> {
        self.mid_to_track.get(&mid).copied()
    }
}
