use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr, time::Instant};

use bytes::Bytes;
use common::v1::types::voice::messages::{
    SfuCommand, SfuEvent, SignallingCommand, SignallingEvent,
};
use common::v1::types::voice::{
    KeyframeRequestKind, MediaKind, Mid, Rid, Speaking, SpeakingWithUserId, TrackKey,
    TrackMetadata, TrackMetadataWithUserId, VoiceState,
};
use common::v1::types::ChannelId;
use common::v2::types::UserId;
use lamprey_backend_core::config::Config;
use slotmap::SlotMap;
use str0m::{media::Direction, Candidate, Event, Output, RtcConfig};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::backend::{BackendConnection, BackendHandle};
use crate::prelude::*;
use crate::util::{PeerId, Subscriber, TrackId, TrackSfu, TrackState};
use crate::webrtc::Webrtc;

/// main entry point
pub struct Sfu {
    backend: BackendHandle,
    shards: HashMap<ChannelId, mpsc::UnboundedSender<ShardCommand>>,
    user_to_channel: HashMap<UserId, ChannelId>,
}

/// a single cpu core managing an entire channel
pub struct SfuShard {
    channel_id: ChannelId,

    /// All active WebRTC connections owned by this shard
    peers: SlotMap<PeerId, Webrtc>,

    /// Map from UserId to PeerId
    user_map: HashMap<UserId, PeerId>,

    /// All tracks published within this shard
    tracks: SlotMap<TrackId, TrackSfu>,

    /// Map incoming IP:Port to PeerId
    addr_map: HashMap<SocketAddr, PeerId>,

    sock_v4: Arc<UdpSocket>,
    sock_v6: Arc<UdpSocket>,

    /// Events to control this shard
    control_rx: mpsc::UnboundedReceiver<ShardCommand>,

    backend: BackendHandle,
}

/// a command sent to a shard
pub enum ShardCommand {
    CreatePeer {
        state: VoiceState,
    },

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
                debug!("SFU Init: {}", sfu_id);
            }
            SfuCommand::CreatePeer {
                state,
                permissions: _,
            } => {
                let channel_id = state.channel_id;
                self.user_to_channel.insert(state.user_id, channel_id);
                let tx = self.get_or_create_shard(channel_id).await;
                let _ = tx.send(ShardCommand::CreatePeer { state });
                // TODO: warn on error
            }
            SfuCommand::Signalling {
                user_id,
                channel_id,
                inner,
            } => {
                if let Some(tx) = self.shards.get(&channel_id) {
                    let _ = tx.send(ShardCommand::Signalling { user_id, inner });
                    // TODO: warn on error
                }
                // TODO: warn on error
            }
            SfuCommand::GenerateKeyframe {
                mid,
                rid,
                kind,
                user_id,
            } => {
                if let Some(channel_id) = self.user_to_channel.get(&user_id).copied() {
                    if let Some(tx) = self.shards.get(&channel_id) {
                        let _ = tx.send(ShardCommand::GenerateKeyframe {
                            user_id,
                            mid,
                            rid,
                            kind,
                        });
                    }
                }
            }
            _ => {
                debug!("unhandled sfu command {:?}", cmd);
            }
        }
    }

    async fn get_or_create_shard(
        &mut self,
        channel_id: ChannelId,
    ) -> mpsc::UnboundedSender<ShardCommand> {
        if let Some(tx) = self.shards.get(&channel_id) {
            return tx.clone();
        }

        // TODO: make sure the socket binds to a working address
        let (tx, rx) = mpsc::unbounded_channel();
        let sock_v4 = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
        let sock_v6 = Arc::new(UdpSocket::bind("[::]:0").await.unwrap());
        debug!(
            "Spawned SfuShard for channel {} on ports {:?}, {:?}",
            channel_id,
            sock_v4.local_addr().unwrap(),
            sock_v6.local_addr().unwrap()
        );

        let backend = self.backend.clone();
        let mut shard = SfuShard {
            channel_id,
            peers: SlotMap::with_key(),
            user_map: HashMap::new(),
            tracks: SlotMap::with_key(),
            addr_map: HashMap::new(),
            sock_v4,
            sock_v6,
            control_rx: rx,
            backend,
        };

        tokio::spawn(async move {
            shard.run().await;
        });

        self.shards.insert(channel_id, tx.clone());
        tx
    }
}

// TODO: allow explicitly subscribing/unsubscribing to audio streams
impl SfuShard {
    pub async fn run(&mut self) {
        let mut buf_v4 = [0u8; 2000];
        let mut buf_v6 = [0u8; 2000];

        loop {
            let mut events_to_dispatch = Vec::new();
            let mut auto_subscriptions = Vec::new();

            // pre-compute publisher user_id for each track to avoid borrow conflicts
            let track_publisher_user: HashMap<TrackId, UserId> = self
                .tracks
                .iter()
                .filter_map(|(tid, t)| self.peers.get(t.publisher).map(|p| (tid, p.user_id)))
                .collect();

            // check if any peer needs negotiation
            for (peer_id, peer) in self.peers.iter_mut() {
                if !peer.pending_tracks.is_empty() {
                    let mut change = peer.rtc.sdp_api();
                    for track_id in peer.pending_tracks.drain(..) {
                        if let Some(track) = self.tracks.get(track_id) {
                            let dir = if track.kind == MediaKind::Audio {
                                Direction::SendOnly
                            } else {
                                Direction::Inactive
                            };
                            let mapped_mid =
                                change.add_media(track.kind.into(), dir, None, None, None);
                            peer.outbound.insert(mapped_mid, track_id);
                            peer.outbound_mid.insert(track_id, mapped_mid);

                            // automatically subscribe to everyone's microphones
                            if track.kind == MediaKind::Audio && track.key == TrackKey::User {
                                auto_subscriptions.push((
                                    track_id,
                                    Subscriber {
                                        peer_id,
                                        sink_mid: mapped_mid,
                                    },
                                ));
                            }
                        }
                    }
                }

                if let Some(mut event) = peer.negotiate_if_needed() {
                    // Populate tracks if it's an offer
                    if let SignallingEvent::Offer { tracks, .. } = &mut event {
                        for (&mapped_mid, &track_id) in &peer.outbound {
                            if let Some(track) = self.tracks.get(track_id) {
                                if let Some(&source_user_id) = track_publisher_user.get(&track_id) {
                                    tracks.push(TrackMetadataWithUserId {
                                        inner: TrackMetadata {
                                            kind: track.kind.into(),
                                            key: track.key.clone(),
                                            mid: mapped_mid.into(),
                                            layers: vec![], // TODO
                                        },
                                        user_id: source_user_id,
                                    });
                                }
                            }
                        }
                    }

                    events_to_dispatch.push((peer.user_id, event));
                }
            }

            for (track_id, sub) in auto_subscriptions {
                if let Some(track) = self.tracks.get_mut(track_id) {
                    track.subscribers.push(sub);
                }
            }

            for (user_id, event) in events_to_dispatch {
                let _ = self.backend.send(SfuEvent::VoiceDispatch {
                    user_id,
                    channel_id: self.channel_id,
                    payload: Box::new(event),
                });
            }

            let peer_keys: Vec<PeerId> = self.peers.keys().collect();
            for peer_id in peer_keys {
                self.drain_peer(peer_id).await;
            }

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
            }
        }
    }

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

        let peer_id = match self.addr_map.get(&source) {
            Some(&id) => Some(id),
            None => {
                // don't know who this is from, find a peer that accepts this input
                let mut found = None;
                for (id, peer) in self.peers.iter_mut() {
                    if peer.rtc.accepts(&input) {
                        found = Some(id);
                        break;
                    }
                }

                if let Some(id) = found {
                    self.addr_map.insert(source, id);
                } else {
                    // TODO: warn?
                }

                found
            }
        };

        if let Some(peer_id) = peer_id {
            if let Err(e) = self.peers[peer_id].rtc.handle_input(input) {
                warn!("Input error: {:?}", e);
            }
        }
    }

    async fn drain_peer(&mut self, peer_id: PeerId) {
        while let Ok(output) = self.peers[peer_id].rtc.poll_output() {
            match output {
                Output::Transmit(t) => {
                    if t.destination.is_ipv4() {
                        let _ = self.sock_v4.send_to(&t.contents, t.destination).await;
                    } else {
                        let _ = self.sock_v6.send_to(&t.contents, t.destination).await;
                    }
                }
                Output::Event(Event::MediaData(media)) => {
                    self.route_media(peer_id, media);
                }
                Output::Event(Event::MediaAdded(m)) => {
                    if let Some(&track_id) = self.peers[peer_id].inbound.get(&m.mid) {
                        self.tracks[track_id].state = TrackState::Open(m.mid);
                    }
                }
                Output::Event(Event::KeyframeRequest(r)) => {
                    if let Some(&track_id) = self.peers[peer_id].outbound.get(&r.mid) {
                        let track = &self.tracks[track_id];
                        let publisher_id = track.publisher;
                        if let Some(mid) = track.state.mid() {
                            if let Some(mut w) = self.peers[publisher_id].rtc.writer(mid) {
                                let _ = w.request_keyframe(r.rid, r.kind);
                            }
                        }
                    }
                }
                Output::Event(Event::ChannelOpen(chan_id, label)) => {
                    if label == "speaking" {
                        self.peers[peer_id].speaking_chan = Some(chan_id);
                    }
                }
                Output::Event(Event::ChannelData(data)) => {
                    if self.peers[peer_id].speaking_chan == Some(data.id) {
                        if let Ok(speaking) = Speaking::from_bytes(&data.data) {
                            let publisher_user_id = self.peers[peer_id].user_id;
                            let track_id = self.peers[peer_id]
                                .inbound
                                .get(&speaking.mid.into())
                                .copied();

                            let peer_keys: Vec<PeerId> = self.peers.keys().collect();
                            for target_peer_id in peer_keys {
                                if target_peer_id == peer_id {
                                    continue;
                                }

                                // PERF: maybe iterate over subscribers directly instead of over peer_keys
                                let mapped_mid = if let Some(t_id) = track_id {
                                    self.tracks[t_id]
                                        .subscribers
                                        .iter()
                                        .find(|s| s.peer_id == target_peer_id)
                                        .map(|s| s.sink_mid.into())
                                        .unwrap_or(speaking.mid)
                                } else {
                                    speaking.mid
                                };

                                let speaking_with_uid = SpeakingWithUserId {
                                    mid: mapped_mid,
                                    flags: speaking.flags,
                                    user_id: publisher_user_id,
                                };
                                let bytes = speaking_with_uid.to_bytes();

                                if let Some(target_peer) = self.peers.get_mut(target_peer_id) {
                                    if let Some(chan) = target_peer.speaking_chan {
                                        if let Some(mut c) = target_peer.rtc.channel(chan) {
                                            let _ = c.write(true, &bytes);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Output::Event(Event::Connected) => {
                    debug!(
                        "Peer {} connected to channel {}",
                        self.peers[peer_id].user_id, self.channel_id
                    );
                }
                Output::Event(_) => {
                    // TODO: handle
                }
                Output::Timeout(_) => {
                    // TODO: handle?
                    break;
                }
            }
        }
    }

    fn route_media(&mut self, publisher_id: PeerId, media: str0m::media::MediaData) {
        let track_id = match self.peers[publisher_id].inbound.get(&media.mid) {
            Some(&id) => id,
            None => return,
        };

        let subscribers = self.tracks[track_id].subscribers.clone();
        for sub in subscribers {
            if sub.peer_id == publisher_id {
                continue;
            }

            if let Some(target_peer) = self.peers.get_mut(sub.peer_id) {
                if let Some(writer) = target_peer.rtc.writer(sub.sink_mid) {
                    let pt = writer.match_params(media.params);

                    if let Some(pt) = pt {
                        let _ =
                            writer.write(pt, media.network_time, media.time, media.data.as_slice());
                    }
                }
            }
        }
    }

    fn handle_command(&mut self, cmd: ShardCommand) {
        match cmd {
            ShardCommand::CreatePeer { state } => {
                let mut rtc_config = RtcConfig::new().set_ice_lite(true).build();

                if let Ok(addr) = self.sock_v4.local_addr() {
                    if let Ok(c) = Candidate::host(addr, "udp") {
                        rtc_config.add_local_candidate(c);
                    }
                }

                if let Ok(addr) = self.sock_v6.local_addr() {
                    if let Ok(c) = Candidate::host(addr, "udp") {
                        rtc_config.add_local_candidate(c);
                    }
                }

                let user_id = state.user_id;
                let peer = Webrtc::new(user_id, state, rtc_config);
                let pid = self.peers.insert(peer);
                self.user_map.insert(user_id, pid);

                let _ = self.backend.send(SfuEvent::PeerCreated {
                    user_id,
                    channel_id: self.channel_id,
                });
            }
            ShardCommand::Signalling { user_id, inner } => {
                if let Some(&pid) = self.user_map.get(&user_id) {
                    let mut tracks_to_have = Vec::new();

                    // update known tracks
                    if let SignallingCommand::Offer { tracks, .. } = &inner {
                        let peer = &mut self.peers[pid];
                        for track in tracks {
                            let track_id = self.tracks.insert(TrackSfu {
                                publisher: pid,
                                subscribers: smallvec::SmallVec::new(),
                                kind: track.kind.into(),
                                key: track.key.clone(),
                                state: TrackState::Negotiating(track.mid.into()),
                            });
                            peer.inbound.insert(track.mid.into(), track_id);

                            tracks_to_have.push((
                                track_id,
                                TrackMetadata {
                                    kind: track.kind.into(),
                                    key: track.key.clone(),
                                    mid: track.mid.into(),
                                    layers: vec![],
                                },
                            ));
                        }
                    }

                    if !tracks_to_have.is_empty() {
                        let peer_keys: Vec<PeerId> = self.peers.keys().collect();
                        for target_pid in peer_keys {
                            if target_pid == pid {
                                continue;
                            }
                            let target_peer = self.peers.get_mut(target_pid).unwrap();
                            let mut have_tracks = Vec::new();
                            for (track_id, track_meta) in &tracks_to_have {
                                target_peer.pending_tracks.push(*track_id);
                                have_tracks.push(track_meta.clone());
                            }

                            let _ = self.backend.send(SfuEvent::VoiceDispatch {
                                user_id: target_peer.user_id,
                                channel_id: self.channel_id,
                                payload: Box::new(SignallingEvent::Have {
                                    user_id, // publisher's user_id
                                    tracks: have_tracks,
                                }),
                            });
                        }
                    }

                    let peer = &mut self.peers[pid];
                    if let SignallingCommand::Want { subscriptions } = &inner {
                        let mut change = peer.rtc.sdp_api();
                        for (&track_id, &mapped_mid) in &peer.outbound_mid {
                            if let Some(track) = self.tracks.get_mut(track_id) {
                                if track.kind == MediaKind::Video {
                                    track.subscribers.retain(|s| s.peer_id != pid);

                                    let mut dir = Direction::Inactive;
                                    if let Some(source_mid) = track.state.mid() {
                                        if subscriptions
                                            .iter()
                                            .any(|sub| sub.mid == source_mid.into())
                                        {
                                            track.subscribers.push(Subscriber {
                                                peer_id: pid,
                                                sink_mid: mapped_mid,
                                            });
                                            dir = Direction::SendOnly;
                                        }
                                    }

                                    change.set_direction(mapped_mid, dir);
                                }
                            }
                        }
                    }

                    if let Some(response_event) = peer.handle_signalling(inner) {
                        let _ = self.backend.send(SfuEvent::VoiceDispatch {
                            user_id,
                            channel_id: self.channel_id,
                            payload: Box::new(response_event),
                        });
                    }
                } else {
                    // TODO: warn
                }
            }
            ShardCommand::GenerateKeyframe {
                user_id,
                mid,
                rid,
                kind,
            } => {
                if let Some(&pid) = self.user_map.get(&user_id) {
                    if let Some(mut w) = self.peers[pid].rtc.writer(mid.into()) {
                        let r = rid.map(|r| r.into());
                        let _ = w.request_keyframe(r, kind.into());
                    }
                }
            }
        }
    }
}
