//! main code for acting as a selective forwarding unit

use crate::{
    backbone::BackboneComms,
    backend::BackendConnection,
    peer::{Command, Peer, PeerEndpoint},
    util::extract_stun_ufrag,
};

use crate::PeerId;
use anyhow::Result;
use bytes::{Bytes, BytesMut};
use common::v1::types::{
    voice::{internal::SfuPermissions, messages::SfuCommand, VoiceState},
    ChannelId, SfuId, UserId,
};
use dashmap::DashMap;
use lamprey_backend_core::config::{Config, ConfigVoice};
use std::hash::Hash;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::task::LocalSet;
use tracing::{debug, warn};

/// shared state
pub struct StateInner {
    pub id: SfuId,
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
    ufrag_to_peer: HashMap<String, PeerId>,
}

pub struct CallHandle {
    users: DashMap<UserId, PeerEndpoint>,
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
    pub fn new(config: Config) -> Self {
        Self {
            state: Arc::new(StateInner {
                id: SfuId::new(),
                voice_config: config
                    .voice
                    .as_ref()
                    .expect("cannot start sfu with no voice config")
                    .clone(),
                config,
            }),
            shards: vec![],
            calls: HashMap::new(),
            ufrag_to_peer: HashMap::new(),
        }
    }

    pub async fn serve(mut self) -> Result<()> {
        let voice_config = &self.state.voice_config;

        let addr_v4 = SocketAddr::new(
            crate::util::select_host_address_ipv4(voice_config.host_ipv4.as_deref())?,
            voice_config.udp_port,
        );
        let sock_v4 = tokio::net::UdpSocket::bind(addr_v4).await?;

        let addr_v6 = SocketAddr::new(
            crate::util::select_host_address_ipv6(voice_config.host_ipv6.as_deref())?,
            voice_config.udp_port,
        );
        let sock_v6 = tokio::net::UdpSocket::bind(addr_v6).await?;

        let mut buf_v4 = BytesMut::with_capacity(2048);
        let mut buf_v6 = BytesMut::with_capacity(2048);

        let mut backbone = BackboneComms::create(Arc::clone(&self.state))?;
        let mut backend = BackendConnection::connect(Arc::clone(&self.state)).await?;

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
                event = backbone.poll() => {
                    let event = Arc::new(event);
                    todo!("send to all shards")
                }
                command = backend.poll() => {
                    if let Ok(cmd) = command {
                        self.handle_command(cmd);
                    }
                }
                Ok((n, source)) = sock_v6.recv_from(&mut buf_v6) => {
                    let packet = buf_v6.split_to(n).freeze();
                    self.handle_packet(source, packet).await;
                }
                Ok((n, source)) = sock_v4.recv_from(&mut buf_v4) => {
                    let packet = buf_v4.split_to(n).freeze();
                    self.handle_packet(source, packet).await;
                }
            }
        }
    }

    /// use STUN demultiplexing to identify and forward packet to the peer
    pub async fn handle_packet(&mut self, source: SocketAddr, data: Bytes) {
        if data.len() >= 20 && (data[0] == 0x00 || data[0] == 0x01) {
            if let Some(ufrag) = extract_stun_ufrag(&data) {
                if let Some(peer_id) = self.ufrag_to_peer.get(&ufrag) {
                    let (channel_id, user_id) = peer_id;
                    let call = self
                        .calls
                        .get(channel_id)
                        .expect("todo better error handling");
                    let peer = call.users.get(user_id).expect("todo better error handling");
                    match &*peer {
                        PeerEndpoint::Webrtc(p) => {
                            debug!(
                                "Routing packet to webrtc peer {:?} from {}",
                                peer_id, source
                            );
                            todo!()
                        }
                        PeerEndpoint::Cascade(_) => {
                            warn!("STUN packet routed to a Cascaded peer, dropping.");
                        }
                    }
                    return;
                }
            }
        }

        warn!("couldn't demultiplex udp packet")
    }

    fn handle_command(&mut self, command: SfuCommand) {
        match command {
            SfuCommand::RecalculateLatency { target_sfu } => todo!("get backbone rtt"),

            SfuCommand::MigrateAll { target_sfu } => todo!("remove this command?"),
            SfuCommand::MigrateUsers {
                users: peers,
                target_sfu,
            } => todo!("remove this command?"),

            SfuCommand::CreatePeer { state, permissions } => todo!("create new peer"),
            SfuCommand::PrepareCascade { sfu_id } => todo!("create token/addr, add to backbone"),
            SfuCommand::CreateCascade {
                sfu_id,
                token,
                addr,
            } => todo!("create new peer wrapping backbone"),

            // unsure what to do with these
            SfuCommand::RouteUpdate {
                channel_id,
                destinations,
            } => todo!(),
            SfuCommand::Channel { channel } => todo!(),

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
                // Find the call this user belongs to by searching all calls
                if let Some(channel_id) = self.find_channel_for_user(user_id) {
                    self.peer_send(
                        (channel_id, user_id),
                        Command::GenerateKeyframe { mid, rid, kind },
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
        state: VoiceState,
        permissions: SfuPermissions,
    ) {
        let call = self.calls.entry(channel_id).or_insert_with(|| CallHandle {
            users: DashMap::new(),
        });
        // TODO: initialize peer and add to call.peers
        let _ = (state, permissions);
    }

    fn cascade_prepare(&mut self, sfu_id: SfuId) {
        // let token = "some_random_token".to_string(); // TODO: generate secure token
        // self.state.backbone.add_pending_token(token.clone(), sfu_id);
        // TODO: share token and address with remote SFU via backend
        let _ = sfu_id;
    }

    fn cascade_create(&mut self, sfu_id: SfuId, token: String, addr: SocketAddr) {
        // TODO: call self.state.backbone.connect(...)
        // TODO: spawn PeerCascading and add to self.calls
        let _ = (sfu_id, token, addr);
    }

    /// get the shard id this channel belongs to
    fn get_channel_shard(&self, channel_id: ChannelId) -> ShardId {
        let idx = (channel_id.as_u128() % self.shards.len() as u128) as usize;
        ShardId(idx)
    }
}
