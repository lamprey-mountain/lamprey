use std::{str::FromStr, sync::Arc};

use crate::{
    Result, ServerState, config,
    serve::{self, serve_transport},
    state::Globals,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use common::{
    v1::types::{RoomCreate, RoomType, util::Time},
    v2::types::{SERVER_ROOM_ID, SERVER_USER_ID},
};
use kerosene_services::globals::GlobalsOwned;
use lamprey_backend_core::{
    Error,
    config::{Config, ListenComponent},
    types::admin::{AdminCollectGarbage, AdminCollectGarbageMode, AdminCollectGarbageTarget},
};
use lamprey_backend_data_postgres::{DbRoomCreate, DbUserCreate, data::Database};
use opentelemetry_otlp::WithExportConfig;
use tokio::task::JoinSet;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

/// the api server
pub struct Server {
    globals: GlobalsOwned,
    router: axum::Router,
}

impl Server {
    /// setup a server
    pub async fn init_from_config(config: Config) -> Result<Self> {
        let globals = Globals::init_from_config(config).await?;
        let server = Self::init(globals).await;
        Ok(server)
    }

    pub async fn init(globals: GlobalsOwned) -> Self {
        let state = todo!();
        let router = serve::create_router(Arc::clone(&state));
        Self { state, router }
    }

    pub fn state(&self) -> Arc<ServerState> {
        // Arc::clone(&self.state)
        todo!()
    }

    pub async fn serve(&self) -> Result<()> {
        info!("starting server");

        let mut set = JoinSet::new();

        for config in &self.state.config.listen {
            if config.components.contains(&ListenComponent::Api) {
                let router = self.router.clone();
                let transport = config.transport.clone();
                info!("api listening on {}", transport);
                set.spawn(async move { serve_transport(transport, router).await });
            }
        }

        if set.is_empty() {
            error!("no components enabled for any listeners");
            return Err(Error::BadStatic("no components enabled for any listeners"));
        }

        while let Some(res) = set.join_next().await {
            res.unwrap()?;
        }

        Ok(())
    }
}

pub fn setup_otel(config: &Config) -> Result<()> {
    kerosene_core::observability::init(config)
}

pub async fn gc(state: Arc<ServerState>, targets: &[AdminCollectGarbageTarget]) -> Result<()> {
    let srv = state.services();

    info!("starting garbage collection");

    let res = srv
        .admin
        .collect_garbage(AdminCollectGarbage {
            targets: targets.to_vec(),
            mode: AdminCollectGarbageMode::Mark,
            async_mode: false,
        })
        .await?;

    for s in res.stats {
        info!(
            "marked {} items of type {:?} for deletion",
            s.rows_deleted, s.target
        );
    }

    info!("deleting...");

    srv.admin
        .collect_garbage(AdminCollectGarbage {
            targets: targets.to_vec(),
            mode: AdminCollectGarbageMode::Sweep,
            async_mode: false,
        })
        .await?;

    info!("done!");

    Ok(())
}
