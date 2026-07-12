use std::time::Instant;

use crate::{
    client::webrtc::{
        Webrtc,
        track::{Inbound, Outbound, TrackState},
    },
    prelude::*,
};

use common::{
    v1::types::voice::{
        MediaKind, Mid, Rid, SessionDescription, TrackKey, TrackMetadata,
        messages::SignallingCommand,
    },
    v2::types::{ChannelId, UserId},
};
use slotmap::SlotMap;
use str0m::Rtc;
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
    users: std::collections::HashMap<UserId, PeerSlot>,
    // /// tracks available in this call
    // tracks: SlotMap<TrackSlot, Track>,

    // TODO: split TrackSlot into InboundSlot and OutboundSlot?
    inbound: SlotMap<TrackSlot, Inbound>,   // formerly `tracks`
    outbound: SlotMap<TrackSlot, Outbound>, // formerly `sinks`
}

impl ShardCall {
    pub fn new(channel_id: ChannelId) -> Self {
        Self {
            channel_id,
            peers: SlotMap::with_key(),
            users: std::collections::HashMap::new(),
            inbound: SlotMap::with_key(),
            outbound: SlotMap::with_key(),
        }
    }

    /// create a new peer connected to this call
    pub fn create_peer(&mut self, s: SfuVoiceState, rtc: Rtc) -> PeerSlot {
        let user_id = s.inner.user_id;
        debug!("Creating peer for user: {:?}", user_id);
        let peer = Webrtc::new(rtc, s);
        let slot = self.peers.insert(peer);
        self.users.insert(user_id, slot);
        slot
    }

    /// a signalling command from a peer
    pub fn handle_signalling(&mut self, peer: PeerSlot, cmd: SignallingCommand) {
        // TODO: sfu_old has special handling for Subscribe, Offer, Answer. do i need this?
        if let Some(p) = self.peers.get_mut(peer) {
            match cmd {
                SignallingCommand::Disconnect => p.disconnect(),
                SignallingCommand::VoiceState { state } => p.update_voice_state(state),
                SignallingCommand::Offer { sdp, tracks: _ } => p.handle_offer(sdp),
                SignallingCommand::Answer { sdp } => p.handle_answer(sdp),
                SignallingCommand::Candidate { candidate } => p.handle_candidate(candidate),
                SignallingCommand::Subscribe { subs: _ } => {}
            }
        }
    }

    pub fn handle_signalling_by_user(&mut self, user_id: UserId, cmd: SignallingCommand) {
        if let Some(&peer) = self.users.get(&user_id) {
            self.handle_signalling(peer, cmd);
        }
    }

    /// request a keyframe to be generated
    pub fn generate_keyframe(
        &mut self,
        user_id: UserId,
        mid: Mid,
        rid: Option<Rid>,
        kind: SKeyframeRequestKind,
    ) {
        if let Some(&peer) = self.users.get(&user_id) {
            if let Some(p) = self.peers.get_mut(peer) {
                let mid = mid.into();
                let rid = rid.map(Into::into);
                if let Some(track) = p.mapping().lookup_track(mid) {
                    let _ = p.request_keyframe(track, rid, kind);
                }
            }
        }
    }

    /// handle str0m input for a peer
    pub fn handle_input(&mut self, peer: PeerSlot, input: SInput) {
        if let Some(p) = self.peers.get_mut(peer) {
            if let Err(e) = p.handle_input(input) {
                warn!("Input error: {:?}", e);
            }
        }
    }

    /// get rtc output events
    // TODO: use proper type for return type (instead of tuple)
    pub fn drain(&mut self) -> (Vec<str0m::net::Transmit>, Option<(PeerSlot, Instant)>) {
        // PERF: reuse `Vec`s
        let mut transmits = Vec::new();
        let mut events = Vec::new();
        let mut min_timeout: Option<(PeerSlot, Instant)> = None;

        for (peer_id, p) in self.peers.iter_mut() {
            while let Ok(output) = p.poll_output() {
                match output {
                    SOutput::Transmit(t) => {
                        transmits.push(t);
                    }
                    SOutput::Event(event) => {
                        events.push((peer_id, event));
                    }
                    SOutput::Timeout(instant) => {
                        if let Some((_, min)) = min_timeout {
                            if instant < min {
                                min_timeout = Some((peer_id, instant));
                            }
                        } else {
                            min_timeout = Some((peer_id, instant));
                        }
                        break;
                    }
                }
            }
        }

        for (peer, event) in events {
            self.handle_peer_event(peer, event);
        }

        (transmits, min_timeout)
    }

    pub fn handle_timeout(&mut self, peer: PeerSlot) {
        let now = Instant::now();
        if let Some(p) = self.peers.get_mut(peer) {
            let _ = p.handle_input(SInput::Timeout(now));
        }
    }

    fn handle_peer_event(&mut self, peer: PeerSlot, event: SEvent) {
        let Some(peer) = self.peers.get_mut(peer) else {
            // warn, this should only be called with existing peers
            return;
        };

        match event {
            SEvent::Connected => {
                debug!(channel_id = ?self.channel_id, "Peer connected");
            }
            SEvent::MediaAdded(media) => {
                debug!(channel_id = ?self.channel_id, mid = ?media.mid, "Media added");
                let mid = media.mid.into();
                if let Some(track_id) = peer.mapping().lookup_track(mid) {
                    if let Some(inbound) = self.inbound.get_mut(track_id) {
                        inbound.state = TrackState::Open(mid);
                    }
                }
            }
            SEvent::MediaData(media) => {
                debug!(channel_id = ?self.channel_id, mid = ?media.mid, "Media data");
                let mid = media.mid.into();
                let Some(track_id) = peer.mapping().lookup_track(mid) else {
                    return;
                };

                let (kind, key) = if let Some(inbound) = self.inbound.get(track_id) {
                    (inbound.kind, inbound.key.clone())
                } else {
                    return;
                };

                // permission checks
                let perms = peer.permissions();
                let can_send = match (kind, &key) {
                    (MediaKind::Audio, TrackKey::User) => perms.audio,
                    _ => perms.video,
                };
                if !can_send {
                    return;
                }

                // get subscribers
                let mut subscriber_peers = Vec::new();
                for (outbound_id, outbound) in self.outbound.iter() {
                    if outbound.source == track_id {
                        subscriber_peers.push((outbound.subscriber, outbound_id));
                    }
                }

                for (sub_peer_id, outbound_id) in subscriber_peers {
                    if let Some(target) = self.peers.get_mut(sub_peer_id) {
                        let target_perms = target.permissions();

                        // if target is deafened and track is audio, skip writing
                        if kind == MediaKind::Audio && target_perms.deaf {
                            continue;
                        }

                        target.write_media(outbound_id, &media);
                    }
                }
            }
            SEvent::ChannelOpen(channel_id, label) => {
                debug!(channel_id = ?self.channel_id, dc_id = ?channel_id, label = %label, "Data channel opened");
            }
            SEvent::ChannelData(data) => {
                debug!(channel_id = ?self.channel_id, dc_id = ?data.id, "Data channel data");
            }
            SEvent::ChannelClose(channel_id) => {
                debug!(channel_id = ?self.channel_id, dc_id = ?channel_id, "Data channel closed");
            }
            SEvent::KeyframeRequest(keyframe_request) => {
                debug!(channel_id = ?self.channel_id, mid = ?keyframe_request.mid, "Keyframe request");
                let mid = keyframe_request.mid.into();

                let Some(outbound_track_id) = peer.mapping().lookup_track(mid) else {
                    return;
                };
                let Some(outbound) = self.outbound.get(outbound_track_id) else {
                    return;
                };
                let inbound_track_id = outbound.source;

                let Some(inbound) = self.inbound.get(inbound_track_id) else {
                    return;
                };
                let publisher_id = inbound.publisher;

                if let Some(publisher) = self.peers.get_mut(publisher_id) {
                    let _ = publisher.request_keyframe(
                        inbound_track_id,
                        keyframe_request.rid.map(Into::into),
                        keyframe_request.kind,
                    );
                }
            }

            _ => {}
        }
    }

    // fn process_sdp_negotiations(&mut self) {}

    // fn route_media(&mut self, publisher: Peer, media: str0m::media::MediaData) {}
}
