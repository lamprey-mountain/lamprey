#![allow(unused)] // TEMP: suppress warnings here for now

use std::time::Duration;

use common::v1::types::federation::Hostname;
use common::v1::types::federation::signing::ServerKeySecret;
use moka::future::Cache;
use tokio::sync::RwLock;
use tracing::error;

use crate::prelude::*;
use crate::services::federation::signing::ValidatedKey;

pub mod import;
pub mod net;
pub mod signing;
pub mod sync;

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub api_url: url::Url,
    pub cdn_url: url::Url,
    pub keys: Vec<ValidatedKey>,
}

pub struct ServiceFederation {
    cache: Cache<Hostname, ServerInfo>,
    local_keys: RwLock<Vec<ServerKeySecret>>,
    state: Globals,
}

impl ServiceFederation {
    pub fn new(state: Globals) -> Self {
        let cache: Cache<Hostname, ServerInfo> = Cache::builder()
            .time_to_live(Duration::from_secs(3600))
            .time_to_idle(Duration::from_secs(1800))
            .max_capacity(1000) // NOTE: someone could theoretically spam subdomains and cause cache issues here
            .build();

        Self {
            cache,
            local_keys: RwLock::new(Vec::new()),
            state,
        }
    }

    /// start background tasks for key rotation
    pub fn start_background_tasks(&self) {
        let state = self.state.clone();
        tokio::spawn(async move {
            let srv = state.services();

            if let Err(err) = srv.federation.load_local_keys().await {
                error!("failed to load local federation keys: {err:?}");
            }
            if let Err(err) = srv.federation.regenerate_keys().await {
                error!("failed to regenerate federation keys: {err:?}");
            }

            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                if let Err(err) = srv.federation.regenerate_keys().await {
                    error!("failed to regenerate federation keys: {err:?}");
                }
            }
        });
    }
}
