use common::v1::types::oauth::OidcClaims;
use lamprey_backend_core::config::ConfigInternal;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::prelude::*;

pub struct ServiceConfig {
    state: Globals,
    internal_config: RwLock<Option<ConfigInternal>>,
    loaded_oidc_jwk: RwLock<Option<Arc<LoadedOidcJwt>>>,
}

pub struct LoadedOidcJwt {
    header: jsonwebtoken::Header,
    encoding_key: jsonwebtoken::EncodingKey,
}

impl ServiceConfig {
    pub fn new(state: Globals) -> Self {
        Self {
            state,
            internal_config: RwLock::new(None),
            loaded_oidc_jwk: RwLock::new(None),
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
        *self.loaded_oidc_jwk.write().await = None;
        Ok(())
    }

    pub async fn sign_oidc_claims(&self, claims: &OidcClaims) -> Result<String> {
        let jwk = self.load_oidc_jwk().await?;
        let token = jsonwebtoken::encode(&jwk.header, claims, &jwk.encoding_key)?;
        Ok(token)
    }

    async fn load_oidc_jwk(&self) -> Result<Arc<LoadedOidcJwt>> {
        if let Some(key) = self.loaded_oidc_jwk.read().await.as_ref() {
            return Ok(key.clone());
        }

        let c = self.internal_get().await?;
        let jwk: jsonwebkey::JsonWebKey = serde_json::from_str(&c.oidc_jwk_key)?;
        let pem = jwk.key.to_pem();

        let (encoding_key, alg) = match jwk.algorithm {
            Some(jsonwebkey::Algorithm::ES256) => (
                jsonwebtoken::EncodingKey::from_ec_pem(pem.as_bytes())?,
                jsonwebtoken::Algorithm::ES256,
            ),
            Some(jsonwebkey::Algorithm::RS256) => (
                jsonwebtoken::EncodingKey::from_rsa_pem(pem.as_bytes())?,
                jsonwebtoken::Algorithm::RS256,
            ),
            _ => return Err(Error::Internal("unsupported signing alg".into())),
        };

        let header = jsonwebtoken::Header {
            alg,
            kid: jwk.key_id.clone(),
            ..Default::default()
        };

        let key = Arc::new(LoadedOidcJwt {
            header,
            encoding_key,
        });

        *self.loaded_oidc_jwk.write().await = Some(key.clone());
        Ok(key)
    }
}
