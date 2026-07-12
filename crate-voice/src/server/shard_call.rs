use crate::{
    client::webrtc::{
        Webrtc,
        track::{Inbound, Outbound},
    },
    prelude::*,
};

use common::{
    v1::types::voice::{SessionDescription, TrackMetadata, messages::SignallingCommand},
    v2::types::ChannelId,
};
use slotmap::SlotMap;
use tracing::{debug, warn};

use crate::util::SfuVoiceState;

/// a shard's voice call data
pub struct ShardCall {
    // call: Call,
    channel_id: ChannelId,
    // channel: SfuChannel,
    // channel: Box<SfuChannel>,
    /// peers connected to this call
    peers: SlotMap<PeerSlot, Webrtc>,
    // users: HashMap<UserId, PeerSlot>,
    // /// tracks available in this call
    // tracks: SlotMap<TrackSlot, Track>,

    // TODO: split TrackSlot into InboundSlot and OutboundSlot?
    inbound: SlotMap<TrackSlot, Inbound>,   // formerly `tracks`
    outbound: SlotMap<TrackSlot, Outbound>, // formerly `sinks`
}

impl ShardCall {
    /// create a new peer  connected to this call
    pub fn create_peer(&mut self, s: SfuVoiceState) {
        debug!("Creating peer for user: {:?}", s.inner.user_id);
        // let rtc = str0m::Rtc::builder().build();
        // let peer = Webrtc::new(rtc);
        // self.peers.insert(peer);
    }

    /// a signalling command from a peer
    pub fn handle_signalling(&mut self, peer: PeerSlot, cmd: SignallingCommand) {
        // TODO: sfu_old has special handling for Subscribe, Offer, Answer. do i need this?
        if let Some(p) = self.peers.get_mut(peer) {
            p.handle_signalling(cmd);
        }
    }

    /// request a keyframe to be generated
    pub fn generate_keyframe(
        &mut self,
        // user_id: UserId,
        // mid: Mid,
        // rid: Option<Rid>,
        // kind: KeyframeRequestKind,
    ) {
        todo!()
    }

    /// handle str0m input for a peer
    pub fn handle_input(&mut self, peer: PeerSlot, input: SInput) {
        if let Some(p) = self.peers.get_mut(peer) {
            if !p.accepts(&input) {
                warn!("Input not accepted by RTC");
            }
            // p.rtc.handle_input(input).unwrap();
        }
    }

    /// get rtc output events
    pub fn drain(&mut self) -> Vec<str0m::net::Transmit> {
        let mut transmits = Vec::new();
        for p in self.peers.values_mut() {
            while let Ok(output) = p.poll_output() {
                match output {
                    SOutput::Transmit(t) => {
                        transmits.push(t);
                    }
                    SOutput::Event(event) => {
                        // TODO: Handle event
                        debug!("RTC event: {:?}", event);
                    }
                    SOutput::Timeout(instant) => {
                        // TODO: Handle timeout
                    }
                }
            }
        }
        transmits
    }

    fn handle_peer_event(&mut self, peer: &mut Webrtc, event: SEvent) {
        match event {
            SEvent::Connected => {
                debug!(
                    channel_id = ?self.channel_id,
                    "Peer connected",
                );
            }

            _ => {}
        }
    }

    fn handle_answer(&mut self, peer: PeerSlot, sdp: SessionDescription) {
        todo!()
    }

    fn handle_offer(
        &mut self,
        peer: PeerSlot,
        sdp: SessionDescription,
        tracks: &[TrackMetadata],
    ) -> Option<SessionDescription> {
        todo!()
    }

    // fn process_sdp_negotiations(&mut self) {}

    // fn route_media(&mut self, publisher: Peer, media: str0m::media::MediaData) {}
}
