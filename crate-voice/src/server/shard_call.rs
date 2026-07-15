use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use crate::{
    client::webrtc::{
        PeerChange, Webrtc,
        track::{Inbound, Outbound, TrackState},
    },
    prelude::*,
};

use common::{
    v1::types::voice::{
        MediaKind, Mid, Rid, TrackKey, TrackMetadata, TrackMetadataWithUserId,
        messages::{SignallingCommand, SignallingEvent},
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
    users: HashMap<UserId, PeerSlot>,
    // /// tracks available in this call
    // tracks: SlotMap<TrackSlot, Track>,

    // TODO: split TrackSlot into InboundSlot and OutboundSlot?
    inbound: SlotMap<TrackSlot, Inbound>,   // formerly `tracks`
    outbound: SlotMap<TrackSlot, Outbound>, // formerly `sinks`
    paused: HashSet<PeerSlot>,
}

impl ShardCall {
    pub fn new(channel_id: ChannelId) -> Self {
        Self {
            channel_id,
            peers: SlotMap::with_key(),
            users: HashMap::new(),
            inbound: SlotMap::with_key(),
            outbound: SlotMap::with_key(),
            paused: HashSet::new(),
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
    pub fn handle_signalling(
        &mut self,
        peer: PeerSlot,
        cmd: SignallingCommand,
    ) -> Vec<SignallingEvent> {
        let mut events = Vec::new();
        if let Some(p) = self.peers.get_mut(peer) {
            match cmd {
                SignallingCommand::Disconnect => p.disconnect(),
                SignallingCommand::VoiceState { state } => p.update_voice_state(state),
                SignallingCommand::Offer { sdp, tracks } => {
                    match p.handle_offer(sdp) {
                        Ok(answer) => {
                            events.push(SignallingEvent::Answer { sdp: answer });

                            // process incoming tracks
                            let mut incoming_mids = HashSet::new();
                            let mut implicit_tracks = Vec::new();

                            for track in tracks {
                                let mid: SMid = track.mid.into();
                                incoming_mids.insert(mid);

                                // check if we already have this inbound track
                                let existing_track_id = self.inbound.iter().find_map(|(id, t)| {
                                    if t.publisher == peer && t.state.mid() == Some(mid) {
                                        Some(id)
                                    } else {
                                        None
                                    }
                                });

                                if let Some(track_id) = existing_track_id {
                                    let t = &mut self.inbound[track_id];
                                    t.kind = track.kind;
                                    t.key = track.key.clone();
                                    t.layers = track.layers.clone();
                                    t.state = TrackState::Open(mid);
                                } else {
                                    let track_id = self.inbound.insert(Inbound {
                                        publisher: peer,
                                        kind: track.kind,
                                        key: track.key.clone(),
                                        layers: track.layers.clone(),
                                        state: TrackState::Open(mid),
                                    });

                                    p.mapping_mut().insert(mid, track_id);

                                    if self.inbound[track_id].is_implicit() {
                                        implicit_tracks.push(track_id);
                                    }
                                }
                            }

                            // find tracks that are no longer referenced
                            let mut dead_tracks = Vec::new();
                            for (track_id, t) in self.inbound.iter() {
                                if t.publisher == peer {
                                    if let Some(mid) = t.state.mid() {
                                        if !incoming_mids.contains(&mid) {
                                            dead_tracks.push((track_id, mid));
                                        }
                                    }
                                }
                            }

                            // remove dead tracks
                            for (track_id, mid) in dead_tracks {
                                self.inbound.remove(track_id);
                                p.mapping_mut().remove_by_mid(mid);

                                // the peer is no longer publishing these tracks
                                // remove associated outbound subscriptions
                                let mut dead_outbound = Vec::new();
                                for (out_id, out) in self.outbound.iter() {
                                    if out.source == track_id {
                                        dead_outbound.push(out_id);
                                    }
                                }
                                for out_id in dead_outbound {
                                    self.outbound.remove(out_id);
                                }
                            }

                            // subscribe other peers to implicit tracks
                            for track_id in implicit_tracks {
                                let target_peers: Vec<_> =
                                    self.peers.keys().filter(|&k| k != peer).collect();
                                for target_peer in target_peers {
                                    self.outbound.insert(Outbound {
                                        subscriber: target_peer,
                                        source: track_id,
                                        state: TrackState::Pending,
                                    });
                                }
                            }

                            // TODO: broadcast a Tracks event to everyone in the channel
                        }
                        Err(e) => {
                            warn!("Failed to handle offer: {:?}", e);
                        }
                    }
                }
                SignallingCommand::Answer { sdp } => {
                    p.handle_answer(sdp);

                    // update inbound tracks
                    for (_, t) in self.inbound.iter_mut().filter(|(_, t)| t.publisher == peer) {
                        if let TrackState::Negotiating(mid) = t.state {
                            t.state = TrackState::Open(mid);
                        }
                    }

                    // update outbound tracks
                    let mut outbound_to_remove = Vec::new();
                    for (track_id, out) in self
                        .outbound
                        .iter_mut()
                        .filter(|(_, o)| o.subscriber == peer)
                    {
                        match out.state {
                            TrackState::Negotiating(mid) => out.state = TrackState::Open(mid),
                            TrackState::Closing(_) => {
                                outbound_to_remove.push(track_id);
                            }
                            _ => {}
                        }
                    }

                    // remove based on outbound_to_remove
                    for track_id in outbound_to_remove {
                        self.outbound.remove(track_id);
                        p.mapping_mut().remove_by_track(track_id);
                    }
                }
                SignallingCommand::Candidate { candidate } => p.handle_candidate(candidate),
                SignallingCommand::Subscribe { subs } => {
                    let mut requested_tracks = HashSet::new();
                    for s in subs {
                        if let Some(&publisher_pid) = self.users.get(&s.user_id) {
                            if let Some(track_id) = self.peers[publisher_pid]
                                .mapping()
                                .lookup_track(s.mid.into())
                            {
                                requested_tracks.insert(track_id);
                            }
                        }
                    }

                    // find existing subscriptions
                    let mut current_subs = Vec::new();
                    for (out_id, out) in self.outbound.iter() {
                        if out.subscriber == peer {
                            current_subs.push((out.source, out_id));
                        }
                    }

                    // mark missing as closing
                    for (tid, sid) in current_subs {
                        if !requested_tracks.contains(&tid) {
                            if let Some(out) = self.outbound.get_mut(sid) {
                                if let Some(mid) = out.state.mid() {
                                    out.state = TrackState::Closing(mid);
                                }
                            }
                        }
                        requested_tracks.remove(&tid);
                    }

                    // add new subscriptions
                    for tid in requested_tracks {
                        self.outbound.insert(Outbound {
                            subscriber: peer,
                            source: tid,
                            state: TrackState::Pending,
                        });
                    }
                }
            }
        }
        events
    }

    pub fn handle_signalling_by_user(
        &mut self,
        user_id: UserId,
        cmd: SignallingCommand,
    ) -> Vec<SignallingEvent> {
        if let Some(&peer) = self.users.get(&user_id) {
            self.handle_signalling(peer, cmd)
        } else {
            Vec::new()
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
        self.unpause(peer);
        if let Some(p) = self.peers.get_mut(peer) {
            if let Err(e) = p.handle_input(input) {
                warn!("Input error: {:?}", e);
            }
        }
    }

    /// get rtc output events
    // TODO: use proper type for return type (instead of tuple)
    pub fn drain(&mut self) -> (Vec<str0m::net::Transmit>, Vec<(PeerSlot, Instant)>) {
        // PERF: reuse `Vec`s
        let mut transmits = Vec::new();
        let mut events = Vec::new();
        let mut timeouts = Vec::new();

        for (peer_id, p) in self.peers.iter_mut() {
            if self.paused.contains(&peer_id) {
                continue;
            }

            while let Ok(output) = p.poll_output() {
                match output {
                    SOutput::Transmit(t) => {
                        transmits.push(t);
                    }
                    SOutput::Event(event) => {
                        events.push((peer_id, event));
                    }
                    SOutput::Timeout(instant) => {
                        self.paused.insert(peer_id);
                        timeouts.push((peer_id, instant));
                        break;
                    }
                }
            }
        }

        for (peer, event) in events {
            self.handle_peer_event(peer, event);
        }

        (transmits, timeouts)
    }

    pub fn unpause(&mut self, peer_slot: PeerSlot) {
        self.paused.remove(&peer_slot);
    }

    pub fn handle_timeout(&mut self, peer: PeerSlot) {
        let now = Instant::now();
        if let Some(p) = self.peers.get_mut(peer) {
            let _ = p.handle_input(SInput::Timeout(now));
        }
    }

    fn handle_peer_event(&mut self, peer_slot: PeerSlot, event: SEvent) {
        let Some(peer) = self.peers.get_mut(peer_slot) else {
            // warn, this should only be called with existing peers
            return;
        };

        peer.handle_event(&event);

        match event {
            SEvent::Connected => {
                debug!(channel_id = ?self.channel_id, "Peer connected");
            }

            // media events
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

            // these events are handled inside peer
            SEvent::ChannelOpen(channel_id, label) => {
                debug!(channel_id = ?self.channel_id, dc_id = ?channel_id, label = %label, "Data channel opened");
            }
            SEvent::ChannelClose(channel_id) => {
                debug!(channel_id = ?self.channel_id, dc_id = ?channel_id, "Data channel closed");
            }

            SEvent::ChannelData(data) => {
                debug!(channel_id = ?self.channel_id, dc_id = ?data.id, "Data channel data");
                // TODO: broadcast speaking data to other users
            }

            _ => {}
        }
    }

    /// create a sdp offer for any local track changes on this sfu
    pub fn process_sdp_negotiations(&mut self) -> Vec<(UserId, SignallingEvent)> {
        // PERF: store a list of dirty tracks and/or peers instead of iterating over every outbound track

        let mut changes: HashMap<PeerSlot, Vec<PeerChange>> = HashMap::new();

        // only outbound changes need to be handled, inbound is handled during client negotiation
        for (outbound_slot, outbound) in &self.outbound {
            match outbound.state {
                TrackState::Pending => {
                    let inbound = &self.inbound[outbound.source];
                    changes
                        .entry(outbound.subscriber)
                        .or_default()
                        .push(PeerChange::Open(outbound_slot, inbound.kind));
                }
                TrackState::Closing(mid) => {
                    changes
                        .entry(outbound.subscriber)
                        .or_default()
                        .push(PeerChange::Close(mid));
                }
                _ => {}
            }
        }

        let mut events = vec![];

        // apply changes for each peer
        for (peer_slot, pending) in changes {
            let peer = &mut self.peers[peer_slot];
            match peer.apply_outbound_changes(&pending) {
                Ok(Some((mids, offer))) => {
                    let user_id = peer.user_id();

                    // update outbound tracks' state to Open
                    for (slot, mid) in mids {
                        self.outbound[slot].state = TrackState::Open(mid);
                    }

                    // collect track metadata for SignallingEvent::Offer
                    let mut tracks = vec![];
                    for outbound in self.outbound.values().filter(|o| o.subscriber == peer_slot) {
                        let t = &self.inbound[outbound.source];

                        // TODO: handle outbound with state Closing

                        tracks.push(TrackMetadataWithUserId {
                            inner: TrackMetadata {
                                // TODO: have actual track ids instead of keying by (mid, user_id) (and sometimes (kind, key)) (?)
                                mid: t.state.mid().unwrap().into(),
                                kind: t.kind,
                                key: t.key.clone(),
                                layers: t.layers.clone(),
                                whisper: None,
                            },
                            user_id: self.peers[t.publisher].user_id(),
                        });
                    }

                    events.push((user_id, SignallingEvent::Offer { sdp: offer, tracks }));
                }
                Ok(None) => {}
                Err(_) => todo!("handle error"),
            }
        }

        events
    }
}

// struct ShardCallDrain(Vec<str0m::net::Transmit>, Vec<(PeerSlot, Instant)>)
