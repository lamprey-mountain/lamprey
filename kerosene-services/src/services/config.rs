use common::v1::types::oauth::OidcClaims;
use lamprey_backend_core::config::ConfigInternal;
use tokio::sync::RwLock;

use crate::{prelude::*, services::oauth_provider::LoadedOidcJwt};

pub struct ServiceConfig {
    globals: Globals,
    internal_config: RwLock<Option<ConfigInternal>>,
    loaded_oidc_jwk: RwLock<Option<Arc<LoadedOidcJwt>>>,
}

impl ServiceConfig {
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            internal_config: RwLock::new(None),
            loaded_oidc_jwk: RwLock::new(None),
        }
    }

    /// get the server's internal config
    pub async fn internal_get(&self) -> Result<ConfigInternal> {
        if let Some(config) = self.internal_config.read().await.as_ref() {
            return Ok(config.to_owned());
        }

        let mut data = self.globals.begin_read().await?;
        let config = data
            .config_get()
            .await?
            .ok_or_else(|| Error::Internal("internal config not initialized".to_string()))?;

        *self.internal_config.write().await = Some(config.clone());
        Ok(config)
    }

    /// set the server's internal config
    pub async fn internal_set(&self, cfg: ConfigInternal) -> Result<()> {
        let mut data = self.globals.begin().await?;
        data.config_put(cfg.clone()).await?;
        data.commit().await?;
        *self.internal_config.write().await = Some(cfg);
        *self.loaded_oidc_jwk.write().await = None;
        Ok(())
    }

    pub async fn load_oidc_jwk(&self) -> Result<Arc<LoadedOidcJwt>> {
        if let Some(key) = self.loaded_oidc_jwk.read().await.as_ref() {
            return Ok(key.clone());
        }

        let c = self.internal_get().await?;
        let key = Arc::new(LoadedOidcJwt::parse(&c.oidc_jwk_key)?);
        *self.loaded_oidc_jwk.write().await = Some(key.clone());
        Ok(key)
    }
}
