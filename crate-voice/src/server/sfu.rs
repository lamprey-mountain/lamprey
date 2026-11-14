use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{
    backend::{BackendConnection, BackendHandle},
    mesh::{Mesh, MeshHandle},
    metrics::VoiceMetrics,
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
use tokio::task::{Id, JoinError, JoinSet};
use tracing::{debug, error, info, warn};

/// how long a shard has to live before its exit is not considered a startup failure
const MIN_SHARD_LIFETIME: Duration = Duration::from_secs(5);

/// how many shards may die right after startup before respawning is given up on
const MAX_QUICK_SHARD_DEATHS: u32 = 3;

/// main entry point for the server
pub struct Sfu {
    backend: BackendHandle,
    mesh: MeshHandle,
    /// running shards, keyed by the id of the task running them
    shards: HashMap<Id, SfuShard>,
    shard_tasks: JoinSet<Result<()>>,
    /// shards in a row that died right after they were spawned
    quick_shard_deaths: u32,
    calls: HashMap<ChannelId, Call>,
    user_to_channel: HashMap<UserId, ChannelId>,
    config_full: Box<Config>,
    config: Box<ConfigVoice>,
    metrics: Option<Arc<VoiceMetrics>>,
}

/// a single voice call known by this sfu
///
/// contains routing data and logic for local and remote cascading
pub struct Call {
    shard: ShardHandle,
    /// task id of the shard hosting this call
    shard_task: Id,
}

/// a shard that is currently running
struct SfuShard {
    handle: ShardHandle,
    /// udp port the shard's media socket is bound to
    port: u16,
    spawned_at: Instant,
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
        let metrics = config.otel.enabled.then(|| {
            let metrics = Arc::new(VoiceMetrics::new());
            metrics.register();
            metrics
        });

        let me = Sfu {
            backend,
            mesh,
            shards: HashMap::new(),
            shard_tasks: JoinSet::new(),
            quick_shard_deaths: 0,
            calls: HashMap::new(),
            user_to_channel: HashMap::new(),
            config_full: Box::new(config),
            config: Box::new(voice_config),
            metrics,
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
                    self.handle_shard_exit(res, &mut ports).await;
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
                tracing::info_span!(
                    "sfu.create_peer",
                    channel_id = ?channel_id,
                    user_id = %user_id
                )
                .in_scope(|| {
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
                            let Some((shard_task, shard)) = self.select_shard() else {
                                error!("No shards available to handle CreatePeer");
                                return;
                            };
                            self.calls.insert(
                                channel_id,
                                Call {
                                    shard: shard.clone(),
                                    shard_task,
                                },
                            );
                            shard
                        }
                    };

                    self.user_to_channel.insert(user_id, channel_id);

                    if let Err(e) = shard.create_peer(channel_id, state) {
                        warn!(?channel_id, ?user_id, "Failed to create peer on shard: {e}");
                    }
                });
            }
            SfuCommand::Signalling {
                user_id,
                channel_id,
                inner,
            } => {
                tracing::info_span!(
                    "sfu.signalling",
                    channel_id = ?channel_id,
                    user_id = %user_id
                )
                .in_scope(|| {
                    debug!(?user_id, ?channel_id, ?inner, "Received signalling command");
                    if let Some(call) = self.calls.get(&channel_id) {
                        if let Err(e) = call.shard.handle_signalling(channel_id, user_id, inner) {
                            warn!(?channel_id, ?user_id, "Failed to forward signalling: {e}");
                        }
                    } else {
                        warn!("Received signalling for unknown channel {:?}", channel_id);
                    }
                });
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
            match Shard::new(
                self.backend.clone(),
                (*self.config).clone(),
                port,
                self.metrics.clone(),
            )
            .await
            {
                Ok((shard, handle)) => {
                    ports.mark_used(port);
                    info!(port, "Spawned shard");

                    let task = self.shard_tasks.spawn(async move {
                        shard.run().await;
                        Ok(())
                    });
                    self.shards.insert(
                        task.id(),
                        SfuShard {
                            handle,
                            port,
                            spawned_at: Instant::now(),
                        },
                    );

                    if let Some(metrics) = &self.metrics {
                        metrics.shard_started();
                    }

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

    /// pick a shard to host a new call
    // TODO: select shard based on load
    fn select_shard(&self) -> Option<(Id, ShardHandle)> {
        self.shards
            .iter()
            .next()
            .map(|(id, shard)| (*id, shard.handle.clone()))
    }

    /// clean up after a shard task, then replace it
    async fn handle_shard_exit(
        &mut self,
        res: ::core::result::Result<(Id, Result<()>), JoinError>,
        ports: &mut PortAllocator,
    ) {
        let (task_id, reason) = match res {
            Ok((id, Ok(()))) => (id, "exited".to_string()),
            Ok((id, Err(e))) => (id, format!("failed: {e}")),
            Err(e) => (e.id(), format!("panicked: {e}")),
        };

        let Some(shard) = self.shards.remove(&task_id) else {
            return;
        };

        error!(port = shard.port, "Shard task {reason}");
        ports.release(shard.port);

        if let Some(metrics) = &self.metrics {
            metrics.shard_stopped();
        }

        // the peers of this shard are gone with it; route new peers of these calls to a live
        // shard instead of a closed handle
        self.calls.retain(|_, call| call.shard_task != task_id);

        // spawning is broken if shards keep dying right after they start
        if shard.spawned_at.elapsed() < MIN_SHARD_LIFETIME {
            self.quick_shard_deaths += 1;
        } else {
            self.quick_shard_deaths = 0;
        }

        if self.quick_shard_deaths >= MAX_QUICK_SHARD_DEATHS {
            error!("Not respawning shard: shards keep dying right after startup");
            return;
        }

        info!(port = shard.port, "Respawning shard");
        if let Err(e) = self.spawn_shard(ports).await {
            error!("Failed to respawn shard: {e}");
        }
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
