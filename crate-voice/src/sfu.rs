use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
    time::Instant,
};

use bytes::Bytes;
use common::{
    v1::types::voice::{
        KeyframeRequestKind, MediaKind, Mid, Rid, SessionDescription, Speaking, SpeakingWithUserId,
        TrackKey, TrackMetadata, TrackMetadataWithUserId, VoiceState,
        internal::SfuPermissions,
        messages::{SfuCommand, SfuEvent, SignallingCommand, SignallingEvent},
    },
    v2::types::{ChannelId, UserId},
};
use futures_util::future::OptionFuture;
use lamprey_backend_core::config::Config;
use slotmap::SlotMap;
use str0m::{Event, Output, media::Direction};
use tokio::{net::UdpSocket, sync::mpsc};
use tracing::{debug, warn};

use crate::{
    backend::{BackendConnection, BackendHandle},
    peer::{Peer, PeerKind},
    util::{PeerId, Router, SfuVoiceState, Sink, SinkId, Track, TrackId, TrackState},
};

use crate::prelude::*;

/// main entry point
pub struct Sfu {
    backend: BackendHandle,
    shards: HashMap<ChannelId, mpsc::UnboundedSender<ShardCommand>>,
    user_to_channel: HashMap<UserId, ChannelId>,
}

/// a single voice call
pub struct Shard {
    channel_id: ChannelId,

    peers: SlotMap<PeerId, Peer>,
    tracks: SlotMap<TrackId, Track>,
    sinks: SlotMap<SinkId, Sink>,
    users: HashMap<UserId, PeerId>,
    addrs: HashMap<SocketAddr, PeerId>,
    router: Router,

    sock_v4: UdpSocket,
    sock_v6: UdpSocket,
    event_queue: Vec<SfuEvent>,
    backend: BackendHandle,
    control_rx: mpsc::UnboundedReceiver<ShardCommand>,
}

/// a command sent to a shard
pub enum ShardCommand {
    CreatePeer {
        state: VoiceState,
        perms: SfuPermissions,
    },

    /// a signalling command that the user sent
    Signalling {
        user_id: UserId,
        inner: SignallingCommand,
    },

    GenerateKeyframe {
        user_id: UserId,
        mid: Mid,
        rid: Option<Rid>,
        kind: KeyframeRequestKind,
    },
}

impl Sfu {
    pub async fn serve(config: Config) -> Result<()> {
        let mut backend_conn = BackendConnection::connect(&config).await?;
        let backend = backend_conn.handle();

        let mut sfu = Self {
            backend,
            shards: HashMap::new(),
            user_to_channel: HashMap::new(),
        };

        while let Ok(cmd) = backend_conn.poll().await {
            sfu.handle_command(cmd).await;
        }

        Ok(())
    }

    async fn handle_command(&mut self, cmd: SfuCommand) {
        match cmd {
            SfuCommand::Init { sfu_id } => {
                debug!(?sfu_id, "SFU Init");
            }
            SfuCommand::CreatePeer { state, permissions } => {
                let channel_id = state.channel_id;
                let shard_tx = self.init_shard(channel_id).await;
                let _ = shard_tx.send(ShardCommand::CreatePeer {
                    state,
                    perms: permissions,
                });
            }
            SfuCommand::Signalling {
                user_id,
                channel_id,
                inner,
            } => {
                // PERF: don't create shard if it doesn't exist?
                let shard_tx = self.init_shard(channel_id).await;
                let _ = shard_tx.send(ShardCommand::Signalling { user_id, inner });
            }
            SfuCommand::GenerateKeyframe {
                mid,
                rid,
                kind,
                user_id,
            } => {
                if let Some(channel_id) = self.user_to_channel.get(&user_id).copied() {
                    let shard_tx = self.init_shard(channel_id).await;
                    let _ = shard_tx.send(ShardCommand::GenerateKeyframe {
                        user_id,
                        mid,
                        rid,
                        kind,
                    });
                }
            }

            SfuCommand::RecalculateLatency { target_sfu: _ } => {
                // SfuEvent::Latency { target_sfu: (), rtt: () }
                todo!("get latency and send it back via an event")
            }

            // TODO: setting up cascade peers
            // SfuCommand::PrepareCascade { sfu_id } => todo!(),
            // SfuCommand::CreateCascade {
            //     sfu_id,
            //     token,
            //     addr,
            // } => todo!(),

            // ignore for now
            // SfuCommand::MigrateUsers { users, target_sfu } => todo!(),
            // SfuCommand::RouteUpdate {
            //     channel_id,
            //     destinations,
            // } => todo!(),
            // SfuCommand::Channel { channel } => {}
            _ => {}
        }
    }

    async fn init_shard(&mut self, channel_id: ChannelId) -> mpsc::UnboundedSender<ShardCommand> {
        if let Some(tx) = self.shards.get(&channel_id) {
            return tx.clone();
        }

        // TODO: make sure the socket binds to a working address
        let (tx, rx) = mpsc::unbounded_channel();
        let sock_v4 = UdpSocket::bind("0.0.0.0:0")
            .await
            .expect("failed to bind v4");
        let sock_v6 = UdpSocket::bind("[::]:0").await.expect("failed to bind v6");
        debug!(
            "Spawned SfuShard for channel {} on ports {:?}, {:?}",
            channel_id,
            sock_v4.local_addr().unwrap(),
            sock_v6.local_addr().unwrap()
        );

        let backend = self.backend.clone();
        let mut shard = Shard {
            channel_id,
            peers: SlotMap::with_key(),
            tracks: SlotMap::with_key(),
            sinks: SlotMap::with_key(),
            router: Router::default(),
            users: HashMap::new(),
            addrs: HashMap::new(),
            sock_v4,
            sock_v6,
            event_queue: Vec::new(),
            backend,
            control_rx: rx,
        };

        tokio::spawn(async move {
            shard.run().await;
        });

        self.shards.insert(channel_id, tx.clone());
        tx
    }
}

impl Shard {
    pub async fn run(&mut self) {
        let mut buf_v4 = [0u8; 2000];
        let mut buf_v6 = [0u8; 2000];

        loop {
            let dead_peers: Vec<_> = self
                .peers
                .iter()
                .filter(|(_, p)| !p.rtc.is_alive())
                .map(|(id, _)| id)
                .collect();

            for pid in dead_peers {
                self.handle_disconnect(pid);
            }

            self.process_sdp_negotiations();

            let instant = self.drain_all_peers().await;

            for e in self.event_queue.drain(..) {
                let _ = self.backend.send(e);
            }

            let timeout = OptionFuture::from(instant.map(|i| tokio::time::sleep_until(i.into())));

            tokio::select! {
                Ok((len, source)) = self.sock_v4.recv_from(&mut buf_v4) => {
                    let packet = Bytes::copy_from_slice(&buf_v4[..len]);
                    self.handle_network_packet(self.sock_v4.local_addr().unwrap(), source, packet, Instant::now()).await;
                }

                Ok((len, source)) = self.sock_v6.recv_from(&mut buf_v6) => {
                    let packet = Bytes::copy_from_slice(&buf_v6[..len]);
                    self.handle_network_packet(self.sock_v6.local_addr().unwrap(), source, packet, Instant::now()).await;
                }

                Some(cmd) = self.control_rx.recv() => {
                    self.handle_command(cmd);
                }

                _ = timeout => {
                    // TODO: send Input::Timeout to str0m?
                }
            }
        }
    }

    fn route_media(&mut self, publisher_id: PeerId, media: str0m::media::MediaData) {
        let peer = &self.peers[publisher_id];
        let Some(track_id) = peer.lookup_track(media.mid) else {
            return;
        };

        let track = &self.tracks[track_id];
        let perms = peer.permissions();

        // enforce publisher permissions
        let can_send = match (track.kind, &track.key) {
            (MediaKind::Audio, TrackKey::User) => perms.audio,
            _ => perms.video,
        };

        if !can_send {
            return;
        }

        let Some(sinks) = self.router.links.get(&track_id) else {
            return;
        };

        for &sink_id in sinks {
            let sink = &self.sinks[sink_id];
            let target = &mut self.peers[sink.subscriber];
            let target_perms = target.permissions();

            // if target is deafened and track is audio, skip writing
            if track.kind == MediaKind::Audio && target_perms.deaf {
                continue;
            }

            if let TrackState::Open(out_mid) = sink.state {
                if let Some(writer) = target.rtc.writer(out_mid) {
                    if let Some(pt) = writer.match_params(media.params) {
                        let _ = writer.write(
                            pt,
                            media.network_time,
                            media.time,
                            Arc::clone(&media.data),
                        );
                    }
                }
            }
        }
    }

    /// handle a udp datagram
    // PERF: prolly should look into that stun/ufrag/whatever thing that i couldnt get working in the old impl
    async fn handle_network_packet(
        &mut self,
        destination: SocketAddr,
        source: SocketAddr,
        data: Bytes,
        now: Instant,
    ) {
        let input = str0m::Input::Receive(
            now,
            str0m::net::Receive {
                proto: str0m::net::Protocol::Udp,
                source,
                destination,
                contents: data.as_ref().try_into().unwrap(),
            },
        );

        let peer_id = match self.addrs.get(&source) {
            Some(&id) => id,
            None => {
                // i don't know which peer this is from...
                // find a peer that accepts this input

                let peer_id = self.peers.iter().find_map(|(peer_id, peer)| {
                    if peer.rtc.accepts(&input) {
                        Some(peer_id)
                    } else {
                        None
                    }
                });

                if let Some(peer_id) = peer_id {
                    self.addrs.insert(source, peer_id);
                    peer_id
                } else {
                    warn!("could not find peer for packet");
                    return;
                }
            }
        };

        if let Err(e) = self.peers[peer_id].rtc.handle_input(input) {
            warn!("Input error: {:?}", e);
        }
    }

    /// drain all peers. returns the instant to wait until.
    async fn drain_all_peers(&mut self) -> Option<Instant> {
        let mut min_instant = None;

        let peer_ids: Vec<PeerId> = self.peers.keys().collect();
        for peer_id in peer_ids {
            if let Some(timeout) = self.drain_peer(peer_id).await {
                match min_instant {
                    Some(t) => {
                        if timeout < t {
                            min_instant = Some(timeout);
                        }
                    }
                    None => min_instant = Some(timeout),
                }
            }
        }

        min_instant
    }

    /// poll and handle every event for a peer. returns the instant to wait until if a timeout is encountered.
    async fn drain_peer(&mut self, peer_id: PeerId) -> Option<Instant> {
        while let Ok(output) = self.peers[peer_id].rtc.poll_output() {
            match output {
                Output::Transmit(t) => {
                    if t.source.is_ipv4() {
                        let _ = self.sock_v4.send_to(&t.contents, t.destination).await;
                    } else {
                        let _ = self.sock_v6.send_to(&t.contents, t.destination).await;
                    }
                }
                Output::Event(e) => self.handle_peer_event(peer_id, e).await,
                Output::Timeout(instant) => {
                    return Some(instant);
                }
            }
        }
        None
    }

    fn handle_disconnect(&mut self, peer_id: PeerId) {
        let Some(peer) = self.peers.remove(peer_id) else {
            return;
        };

        let user_id = peer.user_id();
        debug!(?user_id, ?peer_id, "Peer disconnected");

        if let Some(uid) = user_id {
            self.users.remove(&uid);
        }

        self.addrs.retain(|_, pid| *pid != peer_id);

        let mut tracks_to_remove = Vec::new();
        for (tid, track) in self.tracks.iter() {
            if track.publisher == peer_id {
                tracks_to_remove.push(tid);
            }
        }

        for tid in tracks_to_remove {
            self.tracks.remove(tid);
            // TODO: design Subscriptions in a way that it can be used for negotiation
            // for (_, other_peer) in self.peers.iter_mut() {
            //     other_peer.subscriptions.tracks.remove(&tid);
            // }
        }

        // TODO: send voice state? or disconnect event?
        // SfuEvent::PeerDisconnect { user_id: (), channel_id: () }
    }

    async fn handle_peer_event(&mut self, peer_id: PeerId, event: Event) {
        match event {
            Event::Connected => {
                let peer = &self.peers[peer_id];
                let peer_type = match peer.kind() {
                    PeerKind::User { voice_state, .. } => {
                        format!("User({})", voice_state.user_id)
                    }
                    PeerKind::Cascade { remote_sfu } => {
                        format!("Cascade({})", remote_sfu)
                    }
                };
                debug!(
                    %peer_type,
                    channel_id = ?self.channel_id,
                    "Peer connected",
                );
            }

            Event::MediaAdded(m) => {
                if let Some(track_id) = self.peers[peer_id].lookup_track(m.mid) {
                    self.tracks[track_id].state = TrackState::Open(m.mid);
                }
            }
            Event::MediaData(m) => self.route_media(peer_id, m),
            // Event::MediaChanged(media_changed) => todo!(),
            Event::ChannelOpen(chan_id, label) => {
                // TODO: use protocol instead of label
                // self.peers[peer_id].rtc.channel(chan_id).unwrap().config().unwrap().protocol == "speaking";

                if label == "speaking" {
                    self.peers[peer_id].speaking_chan = Some(chan_id);
                }
            }
            Event::ChannelData(data) => {
                if self.peers[peer_id].speaking_chan == Some(data.id) {
                    if let Ok(speaking) = Speaking::from_bytes(&data.data) {
                        let user_id = self.peers[peer_id]
                            .user_id()
                            .expect("TODO: handle cascading peers sending SpeakingWithUserId?");

                        // map the mid from the publisher's perspective to the track id
                        let Some(track_id) = self.peers[peer_id].lookup_track(speaking.mid.into())
                        else {
                            // NOTE: maybe return an error to the user "cannot send speaking for non existent mid"
                            warn!("speaking mid not found for publisher");
                            return;
                        };

                        let perms = self.peers[peer_id].permissions();
                        let track = &self.tracks[track_id];

                        // enforce publisher permissions
                        let can_send = match (track.kind, &track.key) {
                            (MediaKind::Audio, TrackKey::User) => perms.audio,
                            _ => perms.video,
                        };

                        if !can_send {
                            return;
                        }

                        for (target_peer_id, target_peer) in self.peers.iter_mut() {
                            if target_peer_id == peer_id {
                                continue;
                            }

                            let target_perms = target_peer.permissions();

                            // if target is deafened and track is audio, skip writing speaking indicator
                            if track.kind == MediaKind::Audio && target_perms.deaf {
                                continue;
                            }

                            // map track id to the target peer's mid
                            let Some(&sink_id) =
                                self.router.subscriptions.get(&(target_peer_id, track_id))
                            else {
                                continue;
                            };
                            let Some(target_mid) = self.sinks[sink_id].state.mid() else {
                                continue;
                            };

                            if let Some(chan) = target_peer.speaking_chan {
                                if let Some(mut c) = target_peer.rtc.channel(chan) {
                                    let speaking_with_uid = SpeakingWithUserId {
                                        mid: target_mid.into(),
                                        flags: speaking.flags,
                                        user_id,
                                    };
                                    let _ = c.write(true, &speaking_with_uid.to_bytes());
                                }
                            }
                        }
                    }
                }
            }
            Event::ChannelClose(chan_id) => {
                if self.peers[peer_id].speaking_chan == Some(chan_id) {
                    self.peers[peer_id].speaking_chan = None;
                }
            }

            Event::KeyframeRequest(keyframe_request) => {
                if let Some(track_id) = self.peers[peer_id].lookup_track(keyframe_request.mid) {
                    let track = &self.tracks[track_id];
                    let publisher_peer_id = track.publisher;
                    if let Some(mut w) = self.peers[publisher_peer_id]
                        .rtc
                        .writer(keyframe_request.mid)
                    {
                        let _ = w.request_keyframe(
                            keyframe_request.rid.map(Into::into),
                            keyframe_request.kind.into(),
                        );
                    }
                }
            }

            // TODO: handle other events?
            // Event::IceConnectionStateChange(ice_connection_state) => todo!(),
            // Event::PeerStats(peer_stats) => todo!(),
            // Event::MediaIngressStats(media_ingress_stats) => todo!(),
            // Event::MediaEgressStats(media_egress_stats) => todo!(),
            // Event::EgressBitrateEstimate(bwe_kind) => todo!(),
            // Event::StreamPaused(stream_paused) => todo!(),
            _ => {}
        }
    }

    /// handle a shard command
    fn handle_command(&mut self, cmd: ShardCommand) {
        match cmd {
            ShardCommand::CreatePeer { state, perms } => {
                let mut rtc = str0m::RtcConfig::new()
                    .set_ice_lite(true)
                    .build(Instant::now());

                if let Ok(addr) = self.sock_v4.local_addr() {
                    if let Ok(c) = str0m::Candidate::host(addr, "udp") {
                        rtc.add_local_candidate(c);
                    }
                }

                if let Ok(addr) = self.sock_v6.local_addr() {
                    if let Ok(c) = str0m::Candidate::host(addr, "udp") {
                        rtc.add_local_candidate(c);
                    }
                }

                let user_id = state.user_id;
                let voice_state = SfuVoiceState {
                    inner: state,
                    permissions: perms,
                };
                let peer = Peer::new_user(voice_state, rtc);
                let peer_id = self.peers.insert(peer);
                self.users.insert(user_id, peer_id);
            }
            ShardCommand::Signalling { user_id, inner } => {
                if let Some(&peer_id) = self.users.get(&user_id) {
                    match &inner {
                        // update subscriptions
                        SignallingCommand::Subscribe { subs } => {
                            let mut requested_tracks = HashSet::new();
                            for s in subs {
                                let Some(&publisher_pid) = self.users.get(&s.user_id) else {
                                    continue;
                                };
                                if let Some(track_id) =
                                    self.peers[publisher_pid].lookup_track(s.mid.into())
                                {
                                    requested_tracks.insert(track_id);
                                }
                            }

                            // get current subscriptions for this peer
                            let current_subs: Vec<_> = self
                                .router
                                .subscriptions
                                .iter()
                                .filter(|((pid, _), _)| *pid == peer_id)
                                .map(|((_, tid), sid)| (*tid, *sid))
                                .collect();

                            // remove subscriptions that are no longer requested
                            for (tid, sid) in current_subs {
                                if !requested_tracks.contains(&tid) {
                                    if let Some(sink) = self.sinks.get_mut(sid) {
                                        if let Some(mid) = sink.state.mid() {
                                            sink.state = TrackState::Closing(mid);
                                        }
                                    }
                                }
                            }

                            // add new subscriptions
                            for tid in requested_tracks {
                                self.router.subscribe(peer_id, tid, &mut self.sinks);
                            }

                            return;
                        }

                        SignallingCommand::Offer { sdp, tracks } => {
                            if let Some(answer) = self.handle_offer(peer_id, sdp.clone(), &tracks) {
                                self.event_queue.push(SfuEvent::VoiceDispatch {
                                    user_id,
                                    channel_id: self.channel_id,
                                    payload: Box::new(SignallingEvent::Answer { sdp: answer }),
                                });
                            }
                            return;
                        }

                        SignallingCommand::Answer { sdp } => {
                            self.handle_answer(peer_id, sdp.clone());
                            return;
                        }

                        _ => {}
                    }

                    let events = self.peers[peer_id]
                        .handle_signalling(inner)
                        .into_iter()
                        .map(|e| SfuEvent::VoiceDispatch {
                            user_id,
                            channel_id: self.channel_id,
                            payload: Box::new(e),
                        });
                    self.event_queue.extend(events);
                } else {
                    warn!("got signalling event for non existant peer")
                }
            }
            ShardCommand::GenerateKeyframe {
                user_id,
                mid,
                rid,
                kind,
            } => {
                if let Some(&pid) = self.users.get(&user_id) {
                    if let Some(mut w) = self.peers[pid].rtc.writer(mid.into()) {
                        let r = rid.map(|r| r.into());
                        let _ = w.request_keyframe(r, kind.into());
                    }
                }
            }
        }
    }

    pub fn handle_answer(&mut self, peer_id: PeerId, sdp: SessionDescription) {
        let peer = &mut self.peers[peer_id];
        if let Err(e) = peer.signalling.handle_answer(&mut peer.rtc, sdp) {
            warn!("Failed to handle answer: {:?}", e);
        }

        // update tracks (publisher side)
        for (_, track) in self
            .tracks
            .iter_mut()
            .filter(|(_, t)| t.publisher == peer_id)
        {
            match track.state {
                TrackState::Negotiating(mid) => track.state = TrackState::Open(mid),
                _ => {}
            }
        }

        // update sinks (subscriber side)
        let mut sinks_to_remove = Vec::new();
        for (sid, sink) in self
            .sinks
            .iter_mut()
            .filter(|(_, s)| s.subscriber == peer_id)
        {
            match sink.state {
                TrackState::Negotiating(mid) => sink.state = TrackState::Open(mid),
                TrackState::Closing(_) => {
                    sinks_to_remove.push(sid);
                }
                _ => {}
            }
        }

        for sid in sinks_to_remove {
            let sink = self.sinks.remove(sid).unwrap();
            self.router
                .links
                .get_mut(&sink.source)
                .map(|links| links.remove(&sid));
            self.router
                .subscriptions
                .remove(&(sink.subscriber, sink.source));

            // remove mid from peer mapping
            let peer = &mut self.peers[peer_id];
            peer.mid_to_sink.retain(|_, &mut id| id != sid);
        }
    }

    pub fn handle_offer(
        &mut self,
        peer_id: PeerId,
        sdp: SessionDescription,
        tracks: &[TrackMetadata],
    ) -> Option<SessionDescription> {
        let answer = {
            let peer = &mut self.peers[peer_id];
            peer.signalling
                .handle_offer(&mut peer.rtc, sdp)
                .map_err(|e| warn!("Failed to handle offer: {:?}", e))
                .ok()?
        };

        // register new tracks
        for track in tracks {
            let mid: SMid = track.mid.into();

            let existing_track_id = self.tracks.iter().find_map(|(id, t)| {
                if t.publisher == peer_id && t.state.mid() == Some(mid) {
                    Some(id)
                } else {
                    None
                }
            });

            if let Some(track_id) = existing_track_id {
                let t = &mut self.tracks[track_id];
                t.kind = track.kind;
                t.key = track.key.clone();
                t.layers = track.layers.clone();
                t.state = TrackState::Open(mid);
            } else {
                let track_id = self.tracks.insert(Track {
                    publisher: peer_id,
                    kind: track.kind,
                    key: track.key.clone(),
                    layers: track.layers.clone(),
                    state: TrackState::Open(mid),
                });

                {
                    let peer = &mut self.peers[peer_id];
                    peer.mid_to_track.insert(mid, track_id);
                    peer.track_to_mid.insert(track_id, mid);
                }

                // automatically subscribe everyone else to this track if it's user audio
                let is_always_sub = self.tracks[track_id].is_always_subscribed();
                if is_always_sub {
                    let peer_ids: Vec<_> = self.peers.keys().collect();
                    for target_pid in peer_ids {
                        if target_pid != peer_id {
                            self.router.subscribe(target_pid, track_id, &mut self.sinks);
                        }
                    }
                }
            }
        }

        // find tracks that are no longer referenced
        let incoming_mids: HashSet<SMid> = tracks.iter().map(|t| t.mid.into()).collect();
        let mut dead_tracks = Vec::new();

        for (track_id, track) in self.tracks.iter() {
            if track.publisher == peer_id {
                if let Some(mid) = track.state.mid() {
                    if !incoming_mids.contains(&mid) {
                        dead_tracks.push((track_id, mid));
                    }
                }
            }
        }

        // remove dead tracks
        if !dead_tracks.is_empty() {
            for (track_id, mid) in dead_tracks {
                {
                    let peer = &mut self.peers[peer_id];
                    peer.mid_to_track.remove(&mid);
                    peer.track_to_mid.remove(&track_id);
                }

                self.tracks.remove(track_id);

                // also remove any sinks referencing this track
                let sinks_to_cleanup: Vec<_> = self
                    .sinks
                    .iter()
                    .filter(|(_, s)| s.source == track_id)
                    .map(|(sid, _)| sid)
                    .collect();

                for sid in sinks_to_cleanup {
                    let sink = self.sinks.get_mut(sid).unwrap();
                    if let Some(m) = sink.state.mid() {
                        // Transition to closing so process_sdp_negotiations catches it and calls `stop_media`
                        sink.state = TrackState::Closing(m);
                    } else {
                        // If it was Pending, we can safely delete it immediately
                        let sink = self.sinks.remove(sid).unwrap();
                        self.router.links.get_mut(&track_id).map(|l| l.remove(&sid));
                        self.router
                            .subscriptions
                            .remove(&(sink.subscriber, track_id));
                    }
                }
            }
        }

        // subscribe THIS peer to others' audio
        for (tid, track) in self.tracks.iter() {
            if track.publisher != peer_id && track.is_always_subscribed() {
                self.router.subscribe(peer_id, tid, &mut self.sinks);
            }
        }

        // broadcast a Tracks event to everyone in the channel
        let user_id = self.peers[peer_id]
            .user_id()
            .expect("TODO: cascade support");
        let all_peer_tracks: Vec<TrackMetadata> = self
            .tracks
            .iter()
            .filter(|(_, t)| t.publisher == peer_id)
            .filter_map(|(_, t)| {
                t.state.mid().map(|mid| TrackMetadata {
                    kind: t.kind,
                    key: t.key.clone(),
                    mid: mid.into(),
                    layers: t.layers.clone(),
                })
            })
            .collect();

        let event = Box::new(SignallingEvent::Tracks {
            user_id,
            tracks: all_peer_tracks,
        });

        for (id, peer) in self.peers.iter() {
            if id == peer_id {
                continue;
            }

            if let Some(target_user_id) = peer.user_id() {
                self.event_queue.push(SfuEvent::VoiceDispatch {
                    user_id: target_user_id,
                    channel_id: self.channel_id,
                    payload: event.clone(),
                });
            }
        }

        Some(SessionDescription(answer.to_sdp_string()))
    }

    pub fn process_sdp_negotiations(&mut self) {
        let mut changes_per_peer: HashMap<PeerId, Vec<SinkId>> = HashMap::new();

        for (sink_id, sink) in self.sinks.iter() {
            match sink.state {
                TrackState::Pending | TrackState::Closing(_) => {
                    changes_per_peer
                        .entry(sink.subscriber)
                        .or_default()
                        .push(sink_id);
                }
                _ => {}
            }
        }

        for (peer_id, sink_ids) in changes_per_peer {
            let (offer, tracks) = {
                let peer = &mut self.peers[peer_id];
                let mut changes = peer.rtc.sdp_api();

                for sink_id in sink_ids {
                    let sink = &mut self.sinks[sink_id];
                    match sink.state {
                        TrackState::Pending => {
                            let source = &self.tracks[sink.source];
                            let mid = changes.add_media(
                                source.kind.into(),
                                Direction::SendOnly,
                                None,
                                None,
                                None,
                            );

                            sink.state = TrackState::Negotiating(mid);
                            peer.mid_to_sink.insert(mid, sink_id);
                        }
                        TrackState::Closing(mid) => {
                            changes.stop_media(mid);
                        }
                        _ => {}
                    }
                }

                let offer = match peer.signalling.negotiate_if_needed(changes) {
                    Ok(Some(offer)) => SessionDescription(offer.to_sdp_string()),
                    Ok(None) => continue,
                    Err(e) => {
                        warn!("Failed to negotiate: {:?}", e);
                        continue;
                    }
                };

                let user_id = peer.user_id().expect("TODO: cascade support");

                // Collect tracks for the offer
                let mut tracks = vec![];
                for (sink_id, sink) in self.sinks.iter() {
                    if sink.subscriber == peer_id {
                        if let Some(mid) = sink.state.mid() {
                            let source_track = &self.tracks[sink.source];
                            let publisher_uid = self.peers[source_track.publisher]
                                .user_id()
                                .expect("Missing publisher user_id");

                            tracks.push(TrackMetadataWithUserId {
                                inner: TrackMetadata {
                                    kind: source_track.kind,
                                    key: source_track.key.clone(),
                                    mid: mid.into(),
                                    layers: source_track.layers.clone(),
                                },
                                user_id: publisher_uid,
                            });
                        }
                    }
                }
                (offer, tracks)
            };

            let user_id = self.peers[peer_id]
                .user_id()
                .expect("TODO: cascade support");
            self.event_queue.push(SfuEvent::VoiceDispatch {
                user_id,
                channel_id: self.channel_id,
                payload: Box::new(SignallingEvent::Offer { sdp: offer, tracks }),
            });
        }
    }
}
