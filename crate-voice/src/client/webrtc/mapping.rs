use crate::prelude::SMid;
use crate::util::TrackSlot;
use std::collections::HashMap;

/// mappings between ids local to this peer and shared sfu identifiers
///
/// eg. `Mid`s to `TrackSlot`s
#[derive(Debug, Clone, Default)]
pub struct Mapping {
    // mid_to_track: HashMap<SMid, TrackId>,
    // track_to_mid: HashMap<TrackId, SMid>,
    // mid_to_sink: HashMap<SMid, SinkId>,

    // inbound_map: HashMap<SMid, TrackSlot>,
    // inbound_map_reverse: HashMap<TrackSlot, SMid>,
    // outbound_map: HashMap<SMid, TrackSlot>,
    // /// get outbound
    // pub outbound_to_mid: HashMap<TrackSlot, SMid>,
    pub track_to_mid: HashMap<TrackSlot, SMid>,
    pub mid_to_track: HashMap<SMid, TrackSlot>,
}

impl Mapping {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lookup_mid(&self, track: TrackSlot) -> Option<SMid> {
        self.track_to_mid.get(&track).copied()
    }

    pub fn lookup_track(&self, mid: SMid) -> Option<TrackSlot> {
        self.mid_to_track.get(&mid).copied()
    }
}
