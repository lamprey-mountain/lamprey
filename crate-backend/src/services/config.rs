use lamprey_backend_core::config::ConfigInternal;
use tokio::sync::RwLock;

use crate::prelude::*;

pub struct ServiceConfig {
    state: ServerState2Handle,
    internal_config: RwLock<Option<ConfigInternal>>,
}

impl ServiceConfig {
    pub fn new(state: ServerState2Handle) -> Self {
        Self {
            state,
            internal_config: RwLock::new(None),
        }
    }

    /// get the server's internal config
    pub async fn internal_get(&self) -> Result<ConfigInternal> {
        if let Some(config) = self.internal_config.read().await.as_ref() {
            return Ok(config.to_owned());
        }

        let mut data = self.state.begin_read().await?;
        let config = data
            .config_get()
            .await?
            .ok_or_else(|| Error::Internal("internal config not initialized".to_string()))?;

        *self.internal_config.write().await = Some(config.clone());
        Ok(config)
    }

    /// set the server's internal config
    pub async fn internal_set(&self, cfg: ConfigInternal) -> Result<()> {
        let mut data = self.state.begin().await?;
        data.config_put(cfg.clone()).await?;
        data.commit().await?;
        *self.internal_config.write().await = Some(cfg);
        Ok(())
    }
}
