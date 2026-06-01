use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    time::Instant,
};

use bytes::Bytes;
use common::{
    v1::types::voice::{
        internal::SfuPermissions,
        messages::{SfuCommand, SfuEvent, SignallingCommand, SignallingEvent},
        KeyframeRequestKind, MediaKind, Mid, Rid, Speaking, SpeakingWithUserId, TrackKey,
        TrackMetadata, TrackMetadataWithUserId, VoiceState,
    },
    v2::types::{ChannelId, UserId},
};
use futures_util::future::OptionFuture;
use lamprey_backend_core::config::Config;
use slotmap::SlotMap;
use str0m::{media::Direction, Event, Output};
use tokio::{net::UdpSocket, sync::mpsc};
use tracing::{debug, warn};

use crate::{
    backend::{BackendConnection, BackendHandle},
    peer::{Peer, PeerKind},
    util::{PeerId, SfuVoiceState, Track, TrackId, TrackState},
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
    users: HashMap<UserId, PeerId>,
    addrs: HashMap<SocketAddr, PeerId>,

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

            for (peer_id, peer) in self.peers.iter_mut() {
                if peer.subscriptions.dirty {
                    let mut change = peer.rtc.sdp_api();
                    for tid in &peer.subscriptions.tracks {
                        if !peer.mid_map.contains_key(tid) {
                            let track = &self.tracks[*tid];
                            let mid = change.add_media(
                                track.kind.into(),
                                Direction::RecvOnly,
                                None,
                                None,
                                None,
                            );
                            peer.mid_map.insert(*tid, mid);
                            peer.track_map.insert(mid, *tid);
                        }
                    }

                    // TODO: do something with the Result<Option<SdpOffer>>
                    // peer.signalling.negotiate_if_needed(change);

                    peer.subscriptions.dirty = false;
                }

                if let Some(sdp) = peer.negotiate_if_needed() {
                    let user_id = peer
                        .user_id()
                        .expect("TODO: better error handling, cascade support");
                    let mut tracks = vec![];

                    for (_, track) in self.tracks.iter().filter(|(_, t)| t.publisher == peer_id) {
                        let Some(mid) = track.state.mid() else {
                            continue;
                        };

                        tracks.push(TrackMetadataWithUserId {
                            inner: TrackMetadata {
                                kind: track.kind,
                                key: track.key.clone(),
                                mid: mid.into(),
                                layers: track.layers.clone(),
                            },
                            user_id,
                        });
                    }

                    self.event_queue.push(SfuEvent::VoiceDispatch {
                        user_id,
                        channel_id: self.channel_id,
                        payload: Box::new(SignallingEvent::Offer { sdp, tracks }),
                    });
                }
            }

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
        let always_sub = track.is_always_subscribed();
        let perms = peer.permissions();

        // enforce publisher permissions
        let can_send = match (track.kind, &track.key) {
            (MediaKind::Audio, TrackKey::User) => perms.audio,
            _ => perms.video,
        };

        if !can_send {
            return;
        }

        let peer_ids: Vec<PeerId> = self.peers.keys().collect();

        for target_pid in peer_ids {
            if target_pid == publisher_id {
                continue;
            }

            let target = &mut self.peers[target_pid];
            let target_perms = target.permissions();

            // if target is deafened and track is audio, skip writing
            if track.kind == MediaKind::Audio && target_perms.deaf {
                continue;
            }

            // TODO: handle rid/simulcast
            if always_sub || target.subscriptions.tracks.contains(&track_id) {
                target.write_media(track_id, &media);
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
                            let Some(&target_mid) = target_peer.mid_map.get(&track_id) else {
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
                            let mut tracks = HashSet::new();
                            for s in subs {
                                let Some(&peer_id) = self.users.get(&s.user_id) else {
                                    // maybe return an error?
                                    continue;
                                };
                                if let Some(track_id) =
                                    self.peers[peer_id].lookup_track(s.mid.into())
                                {
                                    tracks.insert(track_id);
                                }
                            }

                            self.peers[peer_id]
                                .subscriptions
                                .update(subs.clone(), tracks);

                            return;
                        }

                        SignallingCommand::Offer { tracks, .. } => {
                            let peer = &mut self.peers[peer_id];

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

                                    // // If the track was previously inactive, restore it to negotiating state
                                    // if let TrackState::Inactive = t.state {
                                    //     t.state = TrackState::Negotiating(mid);
                                    // }
                                } else {
                                    let track_id = self.tracks.insert(Track {
                                        publisher: peer_id,
                                        kind: track.kind,
                                        key: track.key.clone(),
                                        layers: track.layers.clone(),
                                        state: TrackState::Negotiating(mid),
                                    });

                                    peer.track_map.insert(mid, track_id);
                                    peer.mid_map.insert(track_id, mid);
                                }
                            }

                            // find tracks that are no longer referenced
                            let incoming_mids: HashSet<SMid> =
                                tracks.iter().map(|t| t.mid.into()).collect();
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
                                let mut change = peer.rtc.sdp_api();

                                for (track_id, mid) in dead_tracks {
                                    // if let Some(track) = self.tracks.get_mut(track_id) {
                                    //     track.state = TrackState::Inactive;
                                    // }

                                    peer.track_map.remove(&mid);
                                    peer.mid_map.remove(&track_id);

                                    change.stop_media(mid);

                                    // self.tracks.remove(track_id);

                                    // for (_, other_peer) in self.peers.iter_mut() {
                                    //     other_peer.subscriptions.tracks.remove(&track_id);
                                    // }
                                }

                                // TODO: apply change
                            }

                            // get all tracks from this user/peer
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

                            // broadcast a Tracks event to everyone in the channel
                            let event = Box::new(SignallingEvent::Tracks {
                                user_id,
                                tracks: all_peer_tracks,
                            });

                            for (id, peer) in self.peers.iter() {
                                if id == peer_id {
                                    continue;
                                }

                                // find which user_id this peer has
                                let Some(target_user_id) = peer.user_id() else {
                                    continue;
                                };

                                self.event_queue.push(SfuEvent::VoiceDispatch {
                                    user_id: target_user_id,
                                    channel_id: self.channel_id,
                                    payload: event.clone(),
                                });
                            }

                            // TODO: user audio autosubscribe
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
}
