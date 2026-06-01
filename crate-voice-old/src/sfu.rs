//! main code for acting as a selective forwarding unit

use crate::{
    backbone::{BackboneComms, BackboneEvent},
    backend::BackendConnection,
    peer::{webrtc::PeerWebrtc, Command, CommandFull, Peer, PeerEndpoint},
    util::extract_stun_ufrag,
};

use crate::PeerId;
use anyhow::Result;
use bytes::{Bytes, BytesMut};
use common::v1::types::{
    voice::{
        internal::SfuPermissions,
        messages::{BackboneDatagram, BackboneDispatch, PeerEvent, SfuCommand, SfuEvent},
        Mid, TrackMetadata, VoiceState,
    },
    ChannelId, SfuId, UserId,
};
use dashmap::DashMap;
use lamprey_backend_core::config::{Config, ConfigVoice};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{
    net::UdpSocket,
    sync::{broadcast, mpsc, RwLock},
    task::LocalSet,
};
use tracing::{debug, warn};

/// shared state
pub struct StateInner {
    pub id: RwLock<Option<SfuId>>,
    pub config: Config,
    pub voice_config: ConfigVoice,
}

pub type State = Arc<StateInner>;

/// the main entrypoint. creates one sfu
pub struct Sfu {
    state: State,
    shards: Vec<SfuShard>,
    calls: HashMap<ChannelId, CallHandle>,

    // mapping to help routing
    ufrag_to_peer: Arc<dashmap::DashMap<String, PeerId>>,
    addr_to_peer: Arc<dashmap::DashMap<SocketAddr, PeerId>>,

    sock_v4: Arc<UdpSocket>,
    sock_v6: Arc<UdpSocket>,

    backbone: BackboneComms,
    backbone_rx: mpsc::UnboundedReceiver<BackboneEvent>,
    backend: BackendConnection,
}

pub type CallHandle = Arc<CallHandleInner>;

pub struct CallHandleInner {
    users: DashMap<UserId, PeerEndpoint>,
    pub tracks: DashMap<(UserId, Mid), TrackMetadata>,
    tx: broadcast::Sender<Arc<CommandFull>>,
}

/// a set of tasks pinned to a single core
pub struct SfuShard {
    id: ShardId,

    /// spawn futures here
    set: LocalSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardId(pub usize);

impl Sfu {
    pub async fn serve(config: Config) -> Result<()> {
        let voice_config = config
            .voice
            .as_ref()
            .expect("cannot start sfu with no voice config")
            .clone();

        let addr_v4 = SocketAddr::new(
            crate::util::select_host_address_ipv4(voice_config.host_ipv4.as_deref())?,
            voice_config.udp_port,
        );
        let sock_v4 = Arc::new(UdpSocket::bind(addr_v4).await?);

        let addr_v6 = SocketAddr::new(
            crate::util::select_host_address_ipv6(voice_config.host_ipv6.as_deref())?,
            voice_config.udp_port,
        );
        let sock_v6 = Arc::new(UdpSocket::bind(addr_v6).await?);

        let state = Arc::new(StateInner {
            id: RwLock::new(None),
            voice_config,
            config,
        });

        let (backbone, backbone_rx) = BackboneComms::create(Arc::clone(&state))?;
        let backend = BackendConnection::connect(Arc::clone(&state)).await?;

        let me = Self {
            state,
            shards: vec![],
            calls: HashMap::new(),
            ufrag_to_peer: Arc::new(DashMap::new()),
            addr_to_peer: Arc::new(DashMap::new()),
            sock_v4,
            sock_v6,
            backbone,
            backbone_rx,
            backend,
        };

        me.serve_inner().await
    }

    async fn serve_inner(mut self) -> Result<()> {
        let voice_config = &self.state.voice_config;

        let mut buf_v4 = BytesMut::with_capacity(2048);
        let mut buf_v6 = BytesMut::with_capacity(2048);

        let num_workers = voice_config
            .workers
            .unwrap_or_else(|| num_cpus::get() as u8) as usize;
        for n in 0..num_workers {
            self.shards.push(SfuShard {
                id: ShardId(n),
                set: LocalSet::new(),
            });
        }

        loop {
            buf_v4.resize(2000, 0);
            buf_v6.resize(2000, 0);

            tokio::select! {
                Some(event) = self.backbone_rx.recv() => {
                    self.handle_backbone_event(event)
                }
                Ok(command) = self.backend.poll() => {
                    self.handle_command(command).await;
                }
                Ok((n, source)) = self.sock_v6.recv_from(&mut buf_v6) => {
                    let packet = buf_v6.split_to(n).freeze();
                    self.handle_packet(source, packet).await;
                }
                Ok((n, source)) = self.sock_v4.recv_from(&mut buf_v4) => {
                    let packet = buf_v4.split_to(n).freeze();
                    self.handle_packet(source, packet).await;
                }
            }
        }
    }

    /// handle and forward a packet to peer
    // NOTE: uses STUN demultiplexing to identify and forward packet to the peer
    pub async fn handle_packet(&mut self, source: SocketAddr, data: Bytes) {
        // TODO: make this less janky? maybe use `rtc.accepts(input)` instead?

        if data.len() < 2 {
            return;
        }

        // stun packets always start with 0x00 or 0x01
        let is_stun = data[0] == 0x00 || data[0] == 0x01;

        let peer_id = if is_stun {
            // stun: extract ufrag and learn the ip:Port mapping
            if let Some(ufrag) = extract_stun_ufrag(&data) {
                if let Some(pid_ref) = self.ufrag_to_peer.get(&ufrag) {
                    let pid = *pid_ref;
                    // register/update the source address for future srtp packets
                    self.addr_to_peer.insert(source, pid);
                    Some(pid)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            // srtp/srtcp: look up by previously learned ip:port
            self.addr_to_peer
                .get(&source)
                .map(|ref_multi| *ref_multi.value())
        };

        if let Some((channel_id, user_id)) = peer_id {
            if let Some(call) = self.calls.get(&channel_id) {
                if let Some(peer) = call.users.get(&user_id) {
                    match peer.value() {
                        PeerEndpoint::Webrtc(p) => p.handle_network_packet(source, data),
                        PeerEndpoint::Cascade(_) => warn!("got packet for cascade peer"),
                    }
                }
            }
        } else if is_stun {
            warn!("couldn't demultiplex STUN packet from {}", source);
        }
    }

    fn handle_backbone_event(&mut self, event: BackboneEvent) {
        match event {
            BackboneEvent::Dispatch { dispatch, .. } => match dispatch {
                BackboneDispatch::Keyframe {
                    user_id,
                    mid,
                    rid,
                    kind,
                } => {
                    if let Some(channel_id) = self.find_channel_for_user(user_id) {
                        self.peer_send(
                            (channel_id, user_id),
                            Command::GenerateKeyframe {
                                mid,
                                rid,
                                kind,
                                user_id,
                            },
                        );
                    }
                }
                _ => {}
            },
            BackboneEvent::Datagram(dgram) => match dgram {
                BackboneDatagram::Media(m) => {
                    for call in self.calls.values() {
                        for peer in call.users.iter() {
                            peer.handle_media_data(m.clone());
                        }
                    }
                }
                BackboneDatagram::Speaking(s) => {
                    for call in self.calls.values() {
                        for peer in call.users.iter() {
                            peer.handle_speaking(s.clone());
                        }
                    }
                }
            },
            _ => {}
        }
    }

    async fn handle_command(&mut self, command: SfuCommand) {
        debug!("got command {command:?}");
        match command {
            SfuCommand::Init { sfu_id } => {
                let mut id = self.state.id.write().await;
                *id = Some(sfu_id);
            }

            SfuCommand::RecalculateLatency { target_sfu } => {
                let rtt = self.backbone.get_rtt(&target_sfu);
                debug!("Latency for {}: {:?}", target_sfu, rtt);
            }

            // TODO: remove SfuCommand::MigrateUsers? sfus don't need to do anything for migrations
            SfuCommand::MigrateUsers { .. } => todo!("unsure how to impl this command?"),

            SfuCommand::CreatePeer { state, permissions } => {
                let channel_id = state.channel_id;
                let user_id = state.user_id;
                self.peer_create(channel_id, user_id, state, permissions);
            }
            SfuCommand::PrepareCascade { sfu_id } => {
                self.cascade_prepare(sfu_id);
            }
            SfuCommand::CreateCascade {
                sfu_id,
                token,
                addr,
            } => {
                self.cascade_create(sfu_id, token, addr);
            }

            SfuCommand::RouteUpdate { .. } => todo!("do stuff with cascading peers"),
            SfuCommand::Channel { .. } => todo!("update call?"),

            // forward based on user_id
            SfuCommand::Signalling {
                user_id,
                channel_id,
                inner,
            } => {
                self.peer_send((channel_id, user_id), Command::Signalling(inner));
            }
            SfuCommand::GenerateKeyframe {
                mid,
                rid,
                kind,
                user_id,
            } => {
                // find the call this user belongs to by searching all calls
                if let Some(channel_id) = self.find_channel_for_user(user_id) {
                    self.peer_send(
                        (channel_id, user_id),
                        Command::GenerateKeyframe {
                            mid,
                            rid,
                            kind,
                            user_id,
                        },
                    );
                }
            }
        }
    }

    /// send a command to a peer
    fn peer_send(&self, (channel_id, user_id): PeerId, command: Command) {
        if let Some(call) = self.calls.get(&channel_id) {
            if let Some(peer) = call.users.get(&user_id) {
                (*peer).handle_command(command);
            }
        }
    }

    /// find which call a peer belongs to
    fn find_channel_for_user(&self, user_id: UserId) -> Option<ChannelId> {
        // NOTE: this could be optimized, but does it need to be?
        for (channel_id, call) in self.calls.iter() {
            for uid in call.users.iter().map(|p| *p.key()) {
                if uid == user_id {
                    return Some(*channel_id);
                }
            }
        }
        None
    }

    fn peer_create(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
        state: VoiceState,
        permissions: SfuPermissions,
    ) {
        let call = self.calls.entry(channel_id).or_insert_with(|| {
            Arc::new(CallHandleInner {
                users: DashMap::new(),
                tracks: DashMap::new(),
                tx: broadcast::channel(100).0,
            })
        });

        let sock_v4 = Arc::clone(&self.sock_v4);
        let sock_v6 = Arc::clone(&self.sock_v6);

        let peer = PeerWebrtc::spawn(
            user_id,
            state,
            permissions,
            sock_v4,
            sock_v6,
            call.tx.subscribe(),
            Arc::clone(&call),
        );

        let ufrag_map = Arc::clone(&self.ufrag_to_peer);

        // TODO: spawn below future on local set?
        // self.get_channel_shard(channel_id);
        // self.shards[0].set.spawn_local(future);

        let peer2 = peer.clone();
        let backend2 = self.backend.handle();
        let call2 = Arc::clone(&call);
        tokio::spawn(async move {
            let mut peer = peer2;
            let call = call2;
            let backend = backend2;

            while let Some(event) = peer.poll().await {
                match event {
                    PeerEvent::IceUfrag(ufrag) => {
                        ufrag_map.insert(ufrag, (channel_id, user_id));
                    }
                    PeerEvent::Connected => {
                        debug!("Peer {} connected in channel {}", user_id, channel_id);
                    }
                    PeerEvent::Disconnected => {
                        // TODO: remove ufrag/addr mappings
                        // TODO: remove peer
                        // TODO: remove call tracks from this user
                    }
                    // TODO: log _e
                    PeerEvent::MediaAdded(m) => {
                        call.tracks
                            .insert((m.user_id, m.inner.mid.into()), m.inner.clone());

                        if let Err(_e) = call
                            .tx
                            .send(Arc::new(CommandFull::Inner(Command::MediaAdded(m))))
                        {
                            warn!("failed to send MediaAdded command");
                        }
                    }
                    PeerEvent::MediaData(m) => {
                        if let Err(_e) = call.tx.send(Arc::new(CommandFull::MediaData(m))) {
                            warn!("failed to send MediaData command");
                        }
                    }
                    PeerEvent::Speaking(s) => {
                        if let Err(_e) = call.tx.send(Arc::new(CommandFull::Speaking(s))) {
                            warn!("failed to send Speaking command");
                        }
                    }
                    PeerEvent::Signalling(s) => {
                        if let Err(e) = backend.send(SfuEvent::VoiceDispatch {
                            user_id,
                            channel_id,
                            payload: Box::new(s),
                        }) {
                            warn!("failed to send signalling event to backend: {:?}", e);
                        }
                    }
                    PeerEvent::KeyframeRequest {
                        source_mid,
                        user_id: target_user_id,
                        kind,
                        rid,
                    } => {
                        if let Some(target_peer) = call.users.get(&target_user_id) {
                            target_peer.handle_command(Command::GenerateKeyframe {
                                mid: source_mid.into(),
                                rid: rid.map(|r| r.into()),
                                kind: kind.into(),
                                user_id: target_user_id,
                            });
                        }
                    }
                }
            }

            debug!("Event loop for peer {} ended", user_id);
        });

        call.users.insert(user_id, PeerEndpoint::Webrtc(peer));
    }

    fn cascade_prepare(&mut self, sfu_id: SfuId) {
        let token: String = std::iter::repeat_with(fastrand::alphanumeric)
            .take(32)
            .collect();
        self.backbone.add_pending_token(token.clone(), sfu_id);

        let addr = format!(
            "{}:{}",
            self.state
                .voice_config
                .host_ipv4
                .as_deref()
                .or(self.state.voice_config.host_ipv6.as_deref())
                .unwrap(),
            self.state.voice_config.quic_port
        )
        .parse()
        .unwrap();

        if let Err(e) = self.backend.send(SfuEvent::CascadePrepared {
            sfu_id,
            token,
            addr,
        }) {
            warn!("failed to send CascadePrepared event: {:?}", e);
        }
    }

    fn cascade_create(&mut self, sfu_id: SfuId, token: String, addr: SocketAddr) {
        let mut backbone = self.backbone.clone();
        tokio::spawn(async move {
            if let Err(e) = backbone.connect(addr, token, sfu_id).await {
                warn!("failed to connect to remote sfu {}: {:?}", sfu_id, e);
            }
        });
    }

    /// get the shard id this channel belongs to
    // TODO: use this?
    fn get_channel_shard(&self, channel_id: ChannelId) -> ShardId {
        let idx = (channel_id.as_u128() % self.shards.len() as u128) as usize;
        ShardId(idx)
    }
}
