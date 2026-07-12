/// mappings between ids local to this peer and shared sfu identifiers
///
/// eg. `Mid`s to `TrackSlot`s
#[derive(Debug, Clone)]
pub struct Mapping {
    // mid_to_track: HashMap<SMid, TrackId>,
    // track_to_mid: HashMap<TrackId, SMid>,
    // mid_to_sink: HashMap<SMid, SinkId>,

    // inbound_map: HashMap<SMid, TrackSlot>,
    // inbound_map_reverse: HashMap<TrackSlot, SMid>,
    // outbound_map: HashMap<SMid, TrackSlot>,
    // /// get outbound
    // pub outbound_to_mid: HashMap<TrackSlot, SMid>,
}

impl Mapping {
    pub fn new() -> Self {
        Self {}
    }

    // /// get a track slot from this peer's local mid
    // pub fn lookup_track(&self, mid: SMid) -> Option<TrackSlot> {
    //     todo!()
    // }
}
