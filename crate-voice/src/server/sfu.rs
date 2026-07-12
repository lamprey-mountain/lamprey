use std::collections::HashMap;

use crate::{
    backend::{BackendConnection, BackendHandle},
    mesh::{Mesh, MeshHandle},
    prelude::*,
    server::shard::{Shard, ShardHandle},
    util::SfuVoiceState,
};
use common::{
    v1::types::voice::{
        internal::{SfuChannel, SfuPermissions},
        messages::SfuCommand,
    },
    v2::types::ChannelId,
};
use futures::StreamExt;
use lamprey_backend_core::config::{Config, ConfigVoice};
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

/// main entry point for the server
pub struct Sfu {
    backend: BackendHandle,
    mesh: MeshHandle,
    shards: Vec<ShardHandle>,
    shard_tasks: JoinSet<Result<()>>,
    calls: HashMap<ChannelId, Call>,
    // TODO: add
    // user_to_channel: HashMap<UserId, ChannelId>,
    config_full: Box<Config>,
    config: Box<ConfigVoice>,
}

/// a single voice call known by this sfu
///
/// contains routing data and logic for local and remote cascading
pub struct Call {
    channel: SfuChannel,
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
            calls: HashMap::new(),
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

        info!("Spawning {} shards", num_shards);
        for _ in 0..num_shards {
            if let Err(e) = self.spawn_shard().await {
                error!("Failed to spawn shard: {}", e);
            }
        }

        let backend = self.backend.clone();
        let mut commands = Box::pin(backend.subscribe());

        loop {
            tokio::select! {
                Some(cmd) = commands.next() => {
                    self.handle_command(cmd).await;
                }
                Some(res) = self.shard_tasks.join_next() => {
                    match res {
                        Ok(Err(e)) => error!("Shard task failed: {}", e),
                        Ok(Ok(())) => warn!("Shard task exited unexpectedly"),
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
            SfuCommand::CreatePeer { state, permissions } => {
                // TODO: find which shard should handle this peer

                // TEMP: get first shard for now
                let shard = match self.shards.first() {
                    Some(s) => s.clone(),
                    None => {
                        error!("No shards available to handle CreatePeer");
                        return;
                    }
                };

                shard.create_peer(SfuVoiceState {
                    inner: state,
                    permissions,
                });
            }
            // TODO: handle more commands
            _ => {
                warn!("Unhandled SfuCommand");
            }
        }
    }

    async fn spawn_shard(&mut self) -> Result<()> {
        let (shard, handle) = Shard::new(self.backend.clone()).await?;

        self.shard_tasks.spawn(async move {
            shard.run().await;
            Ok(())
        });

        self.shards.push(handle);
        Ok(())
    }
}

impl Call {
    // fn new() -> Self { todo!() }
}

impl SfuHandle {
    /// cleanly shutdown this sfu
    pub async fn shutdown(self) -> Result<()> {
        todo!()
    }

    // fn metrics(&self) -> ...
}
