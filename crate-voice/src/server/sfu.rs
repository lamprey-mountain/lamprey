use std::collections::HashMap;

use crate::{
    mesh::{Mesh, MeshHandle},
    backend::{BackendConnection, BackendHandle},
    prelude::*,
    server::shard::ShardHandle,
};
use common::{
    v1::types::voice::{internal::SfuChannel, messages::SfuCommand},
    v2::types::ChannelId,
};
use lamprey_backend_core::config::{Config, ConfigVoice};
use tokio::task::JoinSet;

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
// TODO: implement
pub struct Call {
    // how should i lay out this struct?
    // inner: Arc<RwLock<CallInner>>,
    // inner: ArcSwap<CallInner>, // arc-swap crate
    // channel_id: ChannelId,
    channel: SfuChannel,
    // channel: Box<SfuChannel>,
    // channel: ArcSwap<SfuChannel>,
    // channel: Cache<SfuChannel>, // with arc swap
    // router: Router,
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

        for _ in 0..num_shards {
            let handle = self.spawn_shard();
            self.shards.push(handle);
        }

        // TODO
    }

    async fn handle_command(&mut self, cmd: SfuCommand) {
        todo!()
    }

    fn spawn_shard(&mut self) -> ShardHandle {
        self.shard_tasks.spawn(async move {
            // TODO
            Ok(())
        });

        todo!()
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
