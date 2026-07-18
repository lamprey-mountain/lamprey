use slotmap::new_key_type;

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

    // TODO: split apart TrackSlot
    // pub struct InboundSlot;
    // pub struct OutboundSlot;

    pub struct CallSlot;
}
