use lamprey_backend_core::config::ConfigInternal;
use tokio::sync::RwLock;

use crate::{prelude::*, services::oauth_provider::LoadedOidcJwt};

pub struct ServiceConfig {
    globals: Globals,
    internal_config: RwLock<Option<ConfigInternal>>,
    loaded_oidc_jwk: RwLock<Option<Arc<LoadedOidcJwt>>>,

    #[cfg(feature = "webtransport")]
    webtransport_identity: wtransport::Identity,
}

impl ServiceConfig {
    pub fn new(globals: Globals) -> Self {
        // TODO: automatically rotate webtransport_identity when it expires
        #[cfg(feature = "webtransport")]
        let webtransport_identity = wt::identity_from_config(globals.config());

        Self {
            globals,
            internal_config: RwLock::new(None),
            loaded_oidc_jwk: RwLock::new(None),

            #[cfg(feature = "webtransport")]
            webtransport_identity,
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

#[cfg(feature = "webtransport")]
mod wt {
    use common::v1::types::{misc::binary::Binary, server::ServerWebtransportCert};
    use lamprey_backend_core::config::Config;
    use sha2::{Digest, Sha256};
    use std::collections::HashSet;
    use wtransport::Identity;

    use crate::services::config::ServiceConfig;

    /// create a new self signed webtransport identity
    ///
    /// The certificate's subject alternative names (SANs) are taken from
    /// config, along with loopback addresses.
    pub(super) fn identity_from_config(config: &Config) -> Identity {
        let wt_host = config
            .webtransport
            .as_ref()
            .and_then(|wt| wt.url.as_ref())
            .and_then(|url| url.host_str());

        let hosts = [
            config.api_url.host_str(),
            config.html_url.host_str(),
            config.hostname.as_deref(),
            wt_host,
        ];

        let sans: HashSet<&str> = ["localhost", "127.0.0.1", "::1"]
            .into_iter()
            .chain(hosts.into_iter().flatten())
            .filter(|h| !h.is_empty())
            .collect();

        Identity::self_signed(sans).expect("failed to generate webtransport identity")
    }

    impl ServiceConfig {
        pub fn webtransport_identity(&self) -> &Identity {
            &self.webtransport_identity
        }

        // PERF: cache this
        pub fn webtransport_cert(&self) -> ServerWebtransportCert {
            let identity = &self.webtransport_identity;
            let cert_chain = identity.certificate_chain();
            let cert_der = cert_chain.as_slice()[0].der();
            let hash = Sha256::digest(cert_der.to_vec());
            let cert = ServerWebtransportCert::Sha256 {
                value: Binary::new(hash.to_vec()).unwrap(),
            };
            cert
        }
    }
}
