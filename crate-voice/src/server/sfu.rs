use std::collections::HashMap;

use crate::{
    backend::{BackendConnection, BackendHandle},
    mesh::{Mesh, MeshHandle},
    prelude::*,
    server::{
        ports::PortAllocator,
        shard::{Shard, ShardHandle},
    },
};
use common::{
    v1::types::voice::messages::SfuCommand,
    v2::types::{ChannelId, UserId},
};
use futures::StreamExt;
use lamprey_backend_core::config::{Config, ConfigVoice};
use tokio::task::{Id, JoinSet};
use tracing::{debug, error, info, warn};

/// main entry point for the server
pub struct Sfu {
    backend: BackendHandle,
    mesh: MeshHandle,
    shards: Vec<ShardHandle>,
    shard_tasks: JoinSet<Result<()>>,
    shard_ports: HashMap<Id, u16>,
    calls: HashMap<ChannelId, Call>,
    user_to_channel: HashMap<UserId, ChannelId>,
    config_full: Box<Config>,
    config: Box<ConfigVoice>,
}

/// a single voice call known by this sfu
///
/// contains routing data and logic for local and remote cascading
pub struct Call {
    shard: ShardHandle,
}

pub struct SfuHandle {
    // TODO
}

impl Sfu {
    pub async fn serve(config: Config) -> Result<SfuHandle> {
        let voice_config = config.voice.clone().expect("voice config required");

        // PERF: init in parallel
        let backend = BackendConnection::connect(config.clone()).await?;
        let mesh = Mesh::spawn(&config).await?;

        let me = Sfu {
            backend,
            mesh,
            shards: Vec::new(),
            shard_tasks: JoinSet::new(),
            shard_ports: HashMap::new(),
            calls: HashMap::new(),
            user_to_channel: HashMap::new(),
            config_full: Box::new(config),
            config: Box::new(voice_config),
        };

        let handle = SfuHandle {
            // TODO
        };

        tokio::spawn(me.run());

        Ok(handle)
    }

    async fn run(mut self) {
        let num_shards = self.config.workers.unwrap_or_else(|| num_cpus::get() as u8);

        let mut ports = match PortAllocator::new(
            self.config.rtp_port_min,
            self.config.rtp_port_max,
            num_shards as usize,
            [self.config.quic_port, self.config.stun_port.unwrap_or(0)],
        ) {
            Ok(ports) => ports,
            Err(e) => {
                error!("Invalid rtp port range: {e}");
                return;
            }
        };

        info!("Spawning {num_shards} shards");
        let mut started = 0;
        for _ in 0..num_shards {
            match self.spawn_shard(&mut ports).await {
                Ok(()) => started += 1,
                Err(e) => error!("Failed to spawn shard: {e}"),
            }
        }

        if started == 0 {
            error!("Failed to spawn any shards");
            return;
        }
        info!("Spawned {started}/{num_shards} shards");

        let backend = self.backend.clone();
        let mut commands = Box::pin(backend.subscribe());

        loop {
            tokio::select! {
                Some(cmd) = commands.next() => {
                    self.handle_command(cmd).await;
                }
                Some(res) = self.shard_tasks.join_next_with_id() => {
                    match res {
                        Ok((id, Ok(()))) => {
                            if let Some(port) = self.shard_ports.remove(&id) {
                                ports.release(port);
                            }
                            warn!("Shard task exited unexpectedly");
                        }
                        Ok((id, Err(e))) => {
                            if let Some(port) = self.shard_ports.remove(&id) {
                                ports.release(port);
                            }
                            error!("Shard task failed: {}", e);
                        }
                        Err(e) => error!("Shard task panicked: {}", e),
                    }
                    // TODO: try to respawn shard
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: SfuCommand) {
        debug!("Received SfuCommand: {:?}", cmd);
        match cmd {
            SfuCommand::Init { sfu_id } => {
                debug!(?sfu_id, "sfu init");
            }
            SfuCommand::CreatePeer { channel_id, state } => {
                let user_id = state.user_id;
                debug!(?channel_id, ?user_id, "Creating peer");

                let shard = match self.calls.get(&channel_id) {
                    Some(call) => {
                        debug!(?channel_id, "Using existing call shard");
                        call.shard.clone()
                    }
                    None => {
                        debug!(?channel_id, "Creating new call for channel");
                        // TODO: select shard based on load
                        // TODO: shut down `ShardCall`s when idle
                        // TODO: update user_to_channel based on voice state, disconnect
                        let shard = match self.shards.first() {
                            Some(s) => s.clone(),
                            None => {
                                error!("No shards available to handle CreatePeer");
                                return;
                            }
                        };
                        self.calls.insert(
                            channel_id,
                            Call {
                                shard: shard.clone(),
                            },
                        );
                        shard
                    }
                };

                self.user_to_channel.insert(user_id, channel_id);

                shard.create_peer(channel_id, state);
            }
            SfuCommand::Signalling {
                user_id,
                channel_id,
                inner,
            } => {
                debug!(?user_id, ?channel_id, ?inner, "Received signalling command");
                if let Some(call) = self.calls.get(&channel_id) {
                    call.shard.handle_signalling(channel_id, user_id, inner);
                } else {
                    warn!("Received signalling for unknown channel {:?}", channel_id);
                }
            }
            // SfuCommand::GenerateKeyframe {
            //     mid,
            //     rid,
            //     kind,
            //     user_id,
            // } => {
            //     if let Some(channel_id) = self.user_to_channel.get(&user_id).copied() {
            //         if let Some(call) = self.calls.get(&channel_id) {
            //             call.shard
            //                 .generate_keyframe(channel_id, user_id, mid, rid, kind.into());
            //         }
            //     }
            // }
            // TODO: handle more commands
            _ => {
                warn!("Unhandled SfuCommand");
            }
        }
    }

    async fn spawn_shard(&mut self, ports: &mut PortAllocator) -> Result<()> {
        for port in ports.candidates() {
            match Shard::new(self.backend.clone(), (*self.config).clone(), port).await {
                Ok((shard, handle)) => {
                    ports.mark_used(port);
                    info!(port, "Spawned shard");

                    let task = self.shard_tasks.spawn(async move {
                        shard.run().await;
                        Ok(())
                    });
                    self.shard_ports.insert(task.id(), port);

                    self.shards.push(handle);
                    return Ok(());
                }
                Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::AddrInUse => {
                    warn!(port, "RTP port already in use, trying next");
                }
                Err(e) => return Err(e),
            }
        }

        Err(Error::Config(
            "exhausted rtp udp port range without binding a shard".into(),
        ))
    }
}

impl Call {
    // fn new() -> Self { todo!() }
}

impl SfuHandle {
    /// cleanly shutdown this sfu
    pub async fn shutdown(self) -> Result<()> {
        // probably use CancellationToken
        todo!()
    }

    // fn metrics(&self) -> ...
}
