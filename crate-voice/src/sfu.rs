use crate::{
    backend::BackendConnection, config::Config, peer::Peer, PeerCommand, PeerEvent,
    PeerEventEnvelope, SignallingMessage, TrackMetadataServer, TrackMetadataSfu,
};
use anyhow::Result;
use bytes::{Bytes, BytesMut};
use common::v1::types::{
    voice::{SfuChannel, SfuCommand, SfuEvent, SfuPermissions, VoiceState},
    ChannelId, SfuId, UserId,
};
use dashmap::DashMap;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    net::SocketAddr,
    sync::Arc,
    thread,
};
use tokio::runtime::Builder;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::LocalSet;
use tracing::{debug, error, info, trace, warn};

use crate::PeerMedia;

#[derive(Debug)]
struct SfuVoiceState {
    state: VoiceState,
    permissions: SfuPermissions,
}

#[derive(Debug, Default)]
struct ChannelState {
    config: Option<SfuChannel>,
    user_ids: HashSet<UserId>,
    tracks: Vec<TrackMetadataSfu>,
    tracks_by_user: HashMap<UserId, Vec<TrackMetadataServer>>,
}

#[derive(Debug)]
pub enum WorkerCommand {
    Signalling {
        user_id: UserId,
        inner: SignallingMessage,
    },
    VoiceState {
        user_id: UserId,
        state: Option<VoiceState>,
        permissions: SfuPermissions,
    },
    Channel {
        channel: SfuChannel,
    },
    SetSfuId(SfuId),
}

pub struct Sfu {
    workers: Vec<UnboundedSender<WorkerCommand>>,
    user_to_channel: DashMap<UserId, ChannelId>,
    sfu_id: Option<SfuId>,
}

impl Sfu {
    pub fn new(workers: Vec<UnboundedSender<WorkerCommand>>) -> Self {
        Self {
            workers,
            user_to_channel: DashMap::new(),
            sfu_id: None,
        }
    }

    pub async fn run(config: Config) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (command_tx, mut command_rx) = mpsc::unbounded_channel();

        let backend = BackendConnection::new(config.clone(), event_rx, command_tx);
        tokio::spawn(backend.spawn());

        let num_workers = config
            .workers
            .map(|w| w as usize)
            .unwrap_or_else(num_cpus::get)
            .max(1);
        info!("Spawning {} SFU workers", num_workers);

        let mut worker_txs = Vec::new();
        for i in 0..num_workers {
            let (w_tx, w_rx) = mpsc::unbounded_channel();
            worker_txs.push(w_tx);
            let config_clone = config.clone();
            let event_tx_clone = event_tx.clone();

            thread::spawn(move || {
                let rt = Builder::new_current_thread().enable_all().build().unwrap();

                let local = LocalSet::new();

                local.block_on(&rt, async move {
                    let mut worker = Worker::new(i, config_clone, w_rx, event_tx_clone).await;
                    if let Err(e) = worker.run().await {
                        error!("Worker {} died: {}", i, e);
                    }
                });
            });
        }

        let mut sfu = Sfu::new(worker_txs);

        loop {
            if let Some(command) = command_rx.recv().await {
                if let Err(err) = sfu.handle_command(command).await {
                    error!("error handling command: {err}");
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: SfuCommand) -> Result<()> {
        trace!("new rpc message {cmd:?}");

        match cmd {
            SfuCommand::Ready { sfu_id } => {
                self.sfu_id = Some(sfu_id);
                for worker in &self.workers {
                    _ = worker.send(WorkerCommand::SetSfuId(sfu_id));
                }
            }
            SfuCommand::Signalling { user_id, inner } => {
                if let Some(channel_id) = self.user_to_channel.get(&user_id) {
                    let worker_idx = self.get_worker_idx(*channel_id);
                    _ = self.workers[worker_idx].send(WorkerCommand::Signalling { user_id, inner });
                } else {
                    warn!("No channel mapping for user {user_id}");
                }
            }
            SfuCommand::VoiceState {
                user_id,
                state,
                permissions,
            } => {
                let old_channel = self.user_to_channel.get(&user_id).map(|c| *c);
                let new_channel = state.as_ref().map(|s| s.channel_id);

                // If moving between channels/workers, clean up old worker
                if let Some(old_c) = old_channel {
                    if new_channel != Some(old_c) {
                        let old_idx = self.get_worker_idx(old_c);
                        _ = self.workers[old_idx].send(WorkerCommand::VoiceState {
                            user_id,
                            state: None,
                            permissions: permissions.clone(),
                        });
                    }
                }

                if let Some(new_c) = new_channel {
                    self.user_to_channel.insert(user_id, new_c);
                    let new_idx = self.get_worker_idx(new_c);
                    _ = self.workers[new_idx].send(WorkerCommand::VoiceState {
                        user_id,
                        state,
                        permissions,
                    });
                } else {
                    self.user_to_channel.remove(&user_id);
                }
            }
            SfuCommand::Channel { channel } => {
                let idx = self.get_worker_idx(channel.id);
                _ = self.workers[idx].send(WorkerCommand::Channel { channel });
            }
        }

        Ok(())
    }

    fn get_worker_idx(&self, channel_id: ChannelId) -> usize {
        (channel_id.as_u128() % self.workers.len() as u128) as usize
    }
}

struct Worker {
    id: usize,
    config: Config,
    command_rx: UnboundedReceiver<WorkerCommand>,
    event_tx: UnboundedSender<SfuEvent>,
    peer_event_rx: UnboundedReceiver<PeerEventEnvelope>,
    peer_event_tx: UnboundedSender<PeerEventEnvelope>,

    peers: HashMap<UserId, (UnboundedSender<PeerCommand>, mpsc::Sender<PeerMedia>)>,
    voice_states: HashMap<UserId, SfuVoiceState>,
    channels: HashMap<ChannelId, ChannelState>,
    sfu_id: Option<SfuId>,

    // Data Plane (Multiplexing)
    socket_v4: Arc<tokio::net::UdpSocket>,
    socket_v6: Arc<tokio::net::UdpSocket>,
    packet_txs: HashMap<UserId, UnboundedSender<(SocketAddr, Bytes)>>,
    addr_to_user: HashMap<SocketAddr, UserId>,
    ufrag_to_user: HashMap<String, UserId>,
}

impl Worker {
    async fn new(
        id: usize,
        config: Config,
        command_rx: UnboundedReceiver<WorkerCommand>,
        event_tx: UnboundedSender<SfuEvent>,
    ) -> Self {
        let (peer_event_tx, peer_event_rx) = mpsc::unbounded_channel();

        // Bind shared UDP sockets with SO_REUSEPORT
        let socket_v4 = {
            use socket2::{Domain, Protocol, Socket, Type};
            let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
            #[cfg(all(unix, not(target_os = "macos")))]
            socket.set_reuse_port(true).unwrap();
            socket.set_reuse_address(true).unwrap();
            socket.set_nonblocking(true).unwrap();
            _ = socket.set_recv_buffer_size(2 * 1024 * 1024);

            let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), config.udp_port);
            socket.bind(&addr.into()).unwrap();
            Arc::new(tokio::net::UdpSocket::from_std(socket.into()).unwrap())
        };

        let socket_v6 = {
            use socket2::{Domain, Protocol, Socket, Type};
            let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP)).unwrap();
            #[cfg(all(unix, not(target_os = "macos")))]
            socket.set_reuse_port(true).unwrap();
            socket.set_reuse_address(true).unwrap();
            socket.set_nonblocking(true).unwrap();
            socket.set_only_v6(true).unwrap();
            _ = socket.set_recv_buffer_size(2 * 1024 * 1024);

            let addr = SocketAddr::new("::".parse().unwrap(), config.udp_port);
            socket.bind(&addr.into()).unwrap();
            Arc::new(tokio::net::UdpSocket::from_std(socket.into()).unwrap())
        };

        Self {
            id,
            config,
            command_rx,
            event_tx,
            peer_event_rx,
            peer_event_tx,
            peers: HashMap::new(),
            voice_states: HashMap::new(),
            channels: HashMap::new(),
            sfu_id: None,
            socket_v4,
            socket_v6,
            packet_txs: HashMap::new(),
            addr_to_user: HashMap::new(),
            ufrag_to_user: HashMap::new(),
        }
    }

    async fn run(&mut self) -> Result<()> {
        info!("Worker {} starting", self.id);
        let mut buf_v4 = BytesMut::with_capacity(2048);
        let mut buf_v6 = BytesMut::with_capacity(2048);

        loop {
            buf_v4.resize(2000, 0);
            buf_v6.resize(2000, 0);

            tokio::select! {
                Some(cmd) = self.command_rx.recv() => {
                    self.handle_command(cmd).await?;
                }
                Some(envelope) = self.peer_event_rx.recv() => {
                    if let Err(err) = self.handle_peer_event(envelope.user_id, envelope.payload).await {
                        error!("Worker {} error handling peer event: {err}", self.id);
                    }
                }
                // Router: Read from shared v4 socket (zero-copy freeze)
                Ok((n, source)) = self.socket_v4.recv_from(&mut buf_v4) => {
                    let packet = buf_v4.split_to(n).freeze();
                    self.route_packet(source, packet).await;
                }
                // Router: Read from shared v6 socket (zero-copy freeze)
                Ok((n, source)) = self.socket_v6.recv_from(&mut buf_v6) => {
                    let packet = buf_v6.split_to(n).freeze();
                    self.route_packet(source, packet).await;
                }
            }
        }
    }

    async fn route_packet(&mut self, source: SocketAddr, data: Bytes) {
        // O(1) route if we already know this address
        if let Some(user_id) = self.addr_to_user.get(&source) {
            if let Some(tx) = self.packet_txs.get(user_id) {
                if tx.send((source, data.clone())).is_ok() {
                    return;
                }
            }
            self.addr_to_user.remove(&source);
        }

        // STUN demultiplexing to avoid broadcasting
        if data.len() >= 20 && (data[0] == 0x00 || data[0] == 0x01) {
            if let Some(ufrag) = self.extract_stun_ufrag(&data) {
                if let Some(user_id) = self.ufrag_to_user.get(&ufrag) {
                    debug!("Registering route for user {} from {}", user_id, source);
                    self.addr_to_user.insert(source, *user_id);
                    if let Some(tx) = self.packet_txs.get(user_id) {
                        _ = tx.send((source, data));
                        return;
                    }
                }
            }
        }

        // Only broadcast if it's not a known ufrag (e.g. initial discovery or non-STUN)
        // This is still a fallback but scoped to unknown ufrags.
        for tx in self.packet_txs.values() {
            _ = tx.send((source, data.clone()));
        }
    }

    fn extract_stun_ufrag(&self, data: &[u8]) -> Option<String> {
        let mut pos = 20; // Skip header
        while pos + 4 <= data.len() {
            let attr_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
            let attr_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
            pos += 4;

            if attr_type == 0x0006 {
                // USERNAME attribute
                if pos + attr_len <= data.len() {
                    let username = String::from_utf8_lossy(&data[pos..pos + attr_len]);
                    // ICE username is usually "local_ufrag:remote_ufrag" or just "local_ufrag"
                    return Some(username.split(':').next().unwrap_or(&username).to_string());
                }
            }
            pos += (attr_len + 3) & !3; // Attributes are padded to 4 bytes
        }
        None
    }

    async fn handle_command(&mut self, cmd: WorkerCommand) -> Result<()> {
        match cmd {
            WorkerCommand::SetSfuId(id) => self.sfu_id = Some(id),
            WorkerCommand::Channel { channel } => {
                let state = self.channels.entry(channel.id).or_default();
                state.config = Some(channel);
            }
            WorkerCommand::Signalling { user_id, inner } => {
                let (state, permissions) = match self.voice_states.get(&user_id) {
                    Some(v) => (v.state.clone(), v.permissions.clone()),
                    None => return Ok(()),
                };

                let (peer_cmd_tx, _) = self.ensure_peer(user_id, &state, &permissions).await?;
                _ = peer_cmd_tx.send(PeerCommand::Signalling(inner));
            }
            WorkerCommand::VoiceState {
                user_id,
                state,
                permissions,
            } => {
                self.handle_voice_state(user_id, state, permissions).await?;
            }
        }
        Ok(())
    }

    async fn handle_voice_state(
        &mut self,
        user_id: UserId,
        state: Option<VoiceState>,
        permissions: SfuPermissions,
    ) -> Result<()> {
        let Some(state) = state else {
            let old = self.voice_states.remove(&user_id);
            if let Some((peer_cmd_tx, _)) = self.peers.remove(&user_id) {
                _ = peer_cmd_tx.send(PeerCommand::Kill);
            }
            self.packet_txs.remove(&user_id);
            self.addr_to_user.retain(|_, v| *v != user_id);
            self.ufrag_to_user.retain(|_, v| *v != user_id);

            if let Some(old_s) = &old {
                if let Some(channel) = self.channels.get_mut(&old_s.state.channel_id) {
                    channel.user_ids.remove(&user_id);
                    channel.tracks.retain(|t| t.peer_id != user_id);
                    channel.tracks_by_user.remove(&user_id);

                    for other_id in &channel.user_ids {
                        if let Some((other_cmd, _)) = self.peers.get(other_id) {
                            _ = other_cmd.send(PeerCommand::UpdateRoutingTable {
                                user_id,
                                signalling_sender: mpsc::unbounded_channel().0,
                                media_sender: mpsc::channel(1).0,
                            });
                        }
                    }
                }
            }

            self.emit(SfuEvent::VoiceState {
                user_id,
                state: None,
                old: old.map(|o| o.state),
            })
            .await?;
            return Ok(());
        };

        debug!("Worker {}: got voice state {state:?}", self.id);
        let channel_id = state.channel_id;

        let (peer_cmd_tx, peer_media_tx) = self.ensure_peer(user_id, &state, &permissions).await?;

        let old = self.voice_states.insert(
            user_id,
            SfuVoiceState {
                state: state.clone(),
                permissions: permissions.clone(),
            },
        );

        if let Some(old_v) = &old {
            if old_v.state.channel_id != channel_id {
                if let Some(old_c) = self.channels.get_mut(&old_v.state.channel_id) {
                    old_c.user_ids.remove(&user_id);
                    old_c.tracks.retain(|t| t.peer_id != user_id);
                    old_c.tracks_by_user.remove(&user_id);
                }
            }
        }

        let channel = self.channels.entry(channel_id).or_default();
        channel.user_ids.insert(user_id);

        _ = peer_cmd_tx.send(PeerCommand::VoiceState(state.clone()));
        _ = peer_cmd_tx.send(PeerCommand::Permissions(permissions.clone()));

        for other_id in &channel.user_ids {
            if *other_id == user_id {
                continue;
            }

            if let Some((other_cmd, other_media)) = self.peers.get(other_id) {
                _ = peer_cmd_tx.send(PeerCommand::UpdateRoutingTable {
                    user_id: *other_id,
                    signalling_sender: other_cmd.clone(),
                    media_sender: other_media.clone(),
                });

                _ = other_cmd.send(PeerCommand::UpdateRoutingTable {
                    user_id,
                    signalling_sender: peer_cmd_tx.clone(),
                    media_sender: peer_media_tx.clone(),
                });
            }
        }

        for track in &channel.tracks {
            _ = peer_cmd_tx.send(PeerCommand::MediaAdded(track.clone()));
        }

        for (peer_id, meta) in &channel.tracks_by_user {
            _ = peer_cmd_tx.send(PeerCommand::Have {
                user_id: *peer_id,
                tracks: meta.clone(),
            });
        }

        self.emit(SfuEvent::VoiceState {
            user_id,
            state: Some(state),
            old: old.map(|o| o.state),
        })
        .await?;

        if let Some(sfu_id) = self.sfu_id {
            self.emit(SfuEvent::VoiceDispatch {
                user_id,
                payload: SignallingMessage::Ready { sfu_id },
            })
            .await?;
        }

        Ok(())
    }

    async fn handle_peer_event(&mut self, user_id: UserId, event: PeerEvent) -> Result<()> {
        match event {
            PeerEvent::IceUfrag(ufrag) => {
                debug!("User {} using ufrag {}", user_id, ufrag);
                self.ufrag_to_user.insert(ufrag, user_id);
            }
            _ => {
                let channel_id = self
                    .voice_states
                    .get(&user_id)
                    .map(|v| v.state.channel_id)
                    .ok_or_else(|| anyhow::anyhow!("No voice state for user"))?;

                match event {
                    PeerEvent::Signalling(payload) => {
                        self.emit(SfuEvent::VoiceDispatch { user_id, payload })
                            .await?;
                    }
                    PeerEvent::MediaAdded(m) => {
                        let channel = self.channels.entry(channel_id).or_default();
                        if channel
                            .tracks
                            .iter()
                            .any(|t| t.source_mid == m.source_mid && t.peer_id == user_id)
                        {
                            return Ok(());
                        }

                        let other_ids: Vec<UserId> = channel
                            .user_ids
                            .iter()
                            .cloned()
                            .filter(|id| *id != user_id)
                            .collect();
                        for other_id in other_ids {
                            if let Some((other_cmd, _)) = self.peers.get(&other_id) {
                                _ = other_cmd.send(PeerCommand::MediaAdded(m.clone()));
                            }
                        }

                        channel.tracks.push(m);
                    }
                    PeerEvent::MediaData(_) => {}
                    PeerEvent::Dead => {
                        self.peers.remove(&user_id);
                        self.packet_txs.remove(&user_id);
                        self.addr_to_user.retain(|_, v| *v != user_id);
                        self.ufrag_to_user.retain(|_, v| *v != user_id);
                        self.voice_states.remove(&user_id);
                        if let Some(channel) = self.channels.get_mut(&channel_id) {
                            channel.user_ids.remove(&user_id);
                            channel.tracks.retain(|t| t.peer_id != user_id);
                            channel.tracks_by_user.remove(&user_id);
                        }
                    }
                    PeerEvent::NeedsKeyframe {
                        source_mid,
                        source_peer,
                        for_peer,
                        kind,
                        rid,
                    } => {
                        if let Some((peer_cmd_tx, _)) = self.peers.get(&source_peer) {
                            _ = peer_cmd_tx.send(PeerCommand::GenerateKeyframe {
                                mid: source_mid,
                                kind,
                                for_peer,
                                rid,
                            });
                        }
                    }
                    PeerEvent::Have { tracks } => {
                        let channel = self.channels.entry(channel_id).or_default();
                        channel.tracks_by_user.insert(user_id, tracks.clone());

                        let other_ids: Vec<UserId> = channel
                            .user_ids
                            .iter()
                            .cloned()
                            .filter(|id| *id != user_id)
                            .collect();
                        for other_id in other_ids {
                            if let Some((other_cmd, _)) = self.peers.get(&other_id) {
                                _ = other_cmd.send(PeerCommand::Have {
                                    user_id,
                                    tracks: tracks.clone(),
                                });
                            }
                        }
                    }
                    PeerEvent::WantHave { user_ids } => {
                        self.handle_want_have(user_id, channel_id, &user_ids)
                            .await?;
                    }
                    PeerEvent::Speaking(speaking) => {
                        if let Some(channel) = self.channels.get(&channel_id) {
                            let other_ids: Vec<UserId> = channel
                                .user_ids
                                .iter()
                                .cloned()
                                .filter(|id| *id != user_id)
                                .collect();
                            for other_id in other_ids {
                                if let Some((other_cmd, _)) = self.peers.get(&other_id) {
                                    _ = other_cmd.send(PeerCommand::Speaking(PeerMedia::Speaking(
                                        speaking.clone(),
                                    )));
                                }
                            }
                        }
                    }
                    PeerEvent::IceUfrag(_) => unreachable!(),
                }
            }
        }
        Ok(())
    }

    async fn handle_want_have(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        user_ids: &[UserId],
    ) -> Result<()> {
        let Some((peer_cmd_tx, _)) = self.peers.get(&user_id) else {
            return Ok(());
        };
        let Some(channel) = self.channels.get(&channel_id) else {
            return Ok(());
        };

        for peer_id in user_ids {
            if let Some(meta) = channel.tracks_by_user.get(peer_id) {
                _ = peer_cmd_tx.send(PeerCommand::Have {
                    user_id: *peer_id,
                    tracks: meta.clone(),
                });
            }
        }

        Ok(())
    }

    async fn ensure_peer(
        &mut self,
        user_id: UserId,
        voice_state: &VoiceState,
        permissions: &SfuPermissions,
    ) -> Result<(UnboundedSender<PeerCommand>, mpsc::Sender<PeerMedia>)> {
        if let Some(peer) = self.peers.get(&user_id) {
            return Ok(peer.clone());
        }

        let (packet_tx, packet_rx) = mpsc::unbounded_channel();
        self.packet_txs.insert(user_id, packet_tx);

        let (peer, peer_cmd_tx, peer_media_tx) = Peer::create(
            &self.config,
            self.peer_event_tx.clone(),
            user_id,
            voice_state.clone(),
            permissions.clone(),
            self.socket_v4.clone(),
            self.socket_v6.clone(),
            packet_rx,
        )
        .await?;

        self.peers
            .insert(user_id, (peer_cmd_tx.clone(), peer_media_tx.clone()));
        tokio::task::spawn_local(peer.run_loop());

        Ok((peer_cmd_tx, peer_media_tx))
    }

    async fn emit(&self, event: SfuEvent) -> Result<()> {
        _ = self.event_tx.send(event);
        Ok(())
    }
}
