use std::{sync::Arc, time::Duration};

use common::v1::types::federation::{Hostname, Remote, ServerKeyAlgorithm, ServerKeys};
use common::v1::types::util::Time;
use common::v1::types::{User, UserId};
use ed25519_dalek::{SigningKey, VerifyingKey};
use lamprey_backend_core::config::ServerKeyInternal;
use moka::future::Cache;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::error;
use url::Url;

use crate::{
    error::{Error, Result},
    ServerStateInner,
};

/// how long to keep expired keys before deleting them
const EXPIRED_KEY_RETENTION: Duration = Duration::from_secs(24 * 3600);

/// key lifetime
const KEY_EXPIRY: Duration = Duration::from_secs(3600);

/// rotate a new key if the freshest one expires within this window
const KEY_ROTATION_WINDOW: Duration = Duration::from_secs(300);

#[derive(Debug, Clone)]
pub enum ValidatedKeyAlgo {
    Ed25519(VerifyingKey),
}

/// a key that hasn't expired and has a valid signature
///
/// extra info is removed to save info
#[derive(Debug, Clone)]
pub struct ValidatedKey {
    pub alg: ValidatedKeyAlgo,
    pub expires_at: Time,
}

#[derive(Debug, Deserialize)]
pub struct WellKnownResponse {
    pub api_url: Url,
    pub cdn_url: Url,
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub api_url: Url,
    pub cdn_url: Url,
    pub keys: Vec<ValidatedKey>,
}

/// a local signing key with its public key pre-parsed
#[derive(Debug, Clone)]
pub struct LocalSigningKey {
    pub pubkey: VerifyingKey,
    pub signing_key: SigningKey,
    pub expires_at: Time,
}

impl LocalSigningKey {
    pub fn from_internal(key: &ServerKeyInternal) -> Result<Self> {
        if key.alg != ServerKeyAlgorithm::Ed25519 {
            return Err(Error::BadStatic("unsupported key algorithm"));
        }

        let pubkey_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            &key.pubkey,
        )
        .map_err(|_| Error::BadStatic("invalid pubkey encoding"))?;

        let privkey_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            &key.privkey,
        )
        .map_err(|_| Error::BadStatic("invalid privkey encoding"))?;

        let pubkey: [u8; 32] = pubkey_bytes
            .try_into()
            .map_err(|_| Error::BadStatic("invalid pubkey length"))?;

        let privkey: [u8; 64] = privkey_bytes
            .try_into()
            .map_err(|_| Error::BadStatic("invalid privkey length"))?;

        let signing_key = SigningKey::from_keypair_bytes(&privkey)
            .map_err(|_| Error::BadStatic("invalid privkey key"))?;
        let pubkey_parsed = VerifyingKey::from_bytes(&pubkey)
            .map_err(|_| Error::BadStatic("invalid public key"))?;

        Ok(Self {
            pubkey: pubkey_parsed,
            signing_key,
            expires_at: key.expires_at,
        })
    }

    pub fn generate_new() -> Self {
        let mut bytes = [0u8; 32];
        rand::fill(&mut bytes);
        let signing_key = SigningKey::from_bytes(&bytes);
        let pubkey = signing_key.verifying_key();
        let expires_at = Time::now_utc() + KEY_EXPIRY;

        Self {
            pubkey,
            signing_key,
            expires_at,
        }
    }

    pub fn to_internal(&self) -> ServerKeyInternal {
        let pubkey_encoded = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            self.pubkey.to_bytes(),
        );

        let privkey_encoded = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            self.signing_key.to_keypair_bytes(),
        );

        ServerKeyInternal {
            alg: ServerKeyAlgorithm::Ed25519,
            pubkey: pubkey_encoded,
            privkey: privkey_encoded,
            expires_at: self.expires_at,
        }
    }
}

impl ValidatedKey {
    /// verify an ed25519 signature
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<()> {
        match &self.alg {
            ValidatedKeyAlgo::Ed25519(verifying_key) => {
                let sig = ed25519_dalek::Signature::from_slice(signature)
                    .map_err(|_| Error::BadStatic("invalid signature encoding"))?;

                verifying_key
                    .verify_strict(message, &sig)
                    .map_err(|_| Error::BadStatic("signature verification failed"))?;

                Ok(())
            }
        }
    }
}

pub struct ServiceFederation {
    cache: Cache<Hostname, ServerInfo>,
    local_keys: RwLock<Vec<LocalSigningKey>>,
    state: Arc<ServerStateInner>,
}

impl ServiceFederation {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
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

    /// load local signing keys from config
    pub async fn load_local_keys(&self) -> Result<()> {
        let config = self
            .state
            .data()
            .config_get()
            .await?
            .ok_or_else(|| Error::Internal("internal config not initialized".to_string()))?;

        let local_keys: Vec<LocalSigningKey> = config
            .federation_keys
            .iter()
            .filter_map(|k| LocalSigningKey::from_internal(k).ok())
            .collect();

        *self.local_keys.write().await = local_keys;
        Ok(())
    }

    /// regenerate local signing keys:
    /// - delete keys expired for EXPIRED_KEY_RETENTION
    /// - create a new key if all keys are expired or the freshest key expires in KEY_ROTATION_WINDOW or less
    pub async fn regenerate_keys(&self) -> Result<()> {
        let now = Time::now_utc();
        let cutoff = now - EXPIRED_KEY_RETENTION;

        let mut local_keys = self.local_keys.write().await;

        local_keys.retain(|k| k.expires_at > cutoff);

        let needs_new_key = local_keys.is_empty()
            || local_keys
                .iter()
                .max_by_key(|k| k.expires_at)
                .map(|k| k.expires_at - now <= KEY_ROTATION_WINDOW)
                .unwrap_or(true);

        if needs_new_key {
            let new_key = LocalSigningKey::generate_new();
            local_keys.push(new_key);

            // TODO: use admin or something as the sole reader/writer for internal config
            let mut config =
                self.state.data().config_get().await?.ok_or_else(|| {
                    Error::Internal("internal config not initialized".to_string())
                })?;

            config.federation_keys = local_keys.iter().map(|k| k.to_internal()).collect();
            self.state.data().config_put(config).await?;
        }

        Ok(())
    }

    /// start background tasks for key rotation
    pub fn start_background_tasks(&self) {
        let state = Arc::clone(&self.state);
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

    /// get the current valid local signing keys
    pub async fn get_local_keys(&self) -> Vec<LocalSigningKey> {
        let now = Time::now_utc();
        self.local_keys
            .read()
            .await
            .iter()
            .filter(|k| k.expires_at > now)
            .cloned()
            .collect()
    }

    /// get the current local signing keys, incuding expired ones
    pub async fn get_all_local_keys(&self) -> Vec<LocalSigningKey> {
        self.local_keys.read().await.clone()
    }

    /// lookup the server info for this hostname
    pub async fn fetch_server_info(&self, hostname: &Hostname) -> Result<ServerInfo> {
        if let Some(info) = self.cache.get(hostname).await {
            return Ok(info);
        }

        let well_known_url = Url::parse(&format!(
            "https://{}/.well-known/lamprey-mountain",
            hostname.0
        ))?;

        let res = self
            .state
            .services()
            .http
            .client
            .get(well_known_url)
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::BadStatic("failed to fetch well-known"));
        }

        let well_known: WellKnownResponse = res.json().await?;

        // TODO: use strongly typed request structs like `common::v1::routes::federation::server_keys_get::Request` instead of manually building urls
        let keys_url = well_known
            .api_url
            .join(&format!("/api/v1/server/{}/keys", &hostname.0))?;

        let res = self
            .state
            .services()
            .http
            .client
            .get(keys_url)
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::BadStatic("failed to fetch server keys"));
        }

        let server_keys: ServerKeys = res.json().await?;

        let now = Time::now_utc();
        let validated: Vec<ValidatedKey> = server_keys
            .keys
            .into_iter()
            .filter(|k| k.expires_at > now)
            .map(|k| {
                let pubkey_bytes = base64::Engine::decode(
                    &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                    &k.pubkey,
                )
                .map_err(|_| Error::BadStatic("invalid pubkey encoding"))?;

                let pubkey_bytes: [u8; 32] = pubkey_bytes
                    .try_into()
                    .map_err(|_| Error::BadStatic("invalid pubkey length"))?;

                let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
                    .map_err(|_| Error::BadStatic("invalid public key"))?;

                Ok(ValidatedKey {
                    alg: ValidatedKeyAlgo::Ed25519(verifying_key),
                    expires_at: k.expires_at,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let info = ServerInfo {
            api_url: well_known.api_url,
            cdn_url: well_known.cdn_url,
            keys: validated,
        };

        self.cache.insert(hostname.to_owned(), info.clone()).await;
        Ok(info)
    }

    /// fetch the signing keys for this hostname
    pub async fn fetch_keys(&self, hostname: &Hostname) -> Result<Vec<ValidatedKey>> {
        let info = self.fetch_server_info(hostname).await?;
        Ok(info.keys)
    }

    /// Load a user from a remote server, fetching and caching it locally.
    pub async fn load_remote_user(&self, user_id: UserId, hostname: &Hostname) -> Result<User> {
        let info = self.fetch_server_info(hostname).await?;
        let url = info.api_url.join(&format!("/api/v1/user/{}", user_id))?;

        let res = self.state.services().http.client.get(url).send().await?;
        if !res.status().is_success() {
            return Err(Error::BadStatic("failed to fetch remote user"));
        }

        let user: serde_json::Value = res.json().await?;
        dbg!(&user);
        let mut user: User = serde_json::from_value(user)?;
        user.remote = Some(Remote {
            origin_id: user_id.into_inner(),
            hostname: hostname.clone(),
        });

        let data = self.state.data();
        if data.user_get(user_id).await.is_ok() {
            data.user_update(
                user_id,
                common::v1::types::UserPatch {
                    name: Some(user.name.clone()),
                    description: Some(user.description.clone()),
                    avatar: Some(user.avatar),
                    banner: Some(user.banner),
                },
            )
            .await?;
        } else {
            data.user_create(crate::types::DbUserCreate {
                id: Some(user_id),
                parent_id: None,
                name: user.name.clone(),
                description: user.description.clone(),
                puppet: user.puppet.clone(),
                registered_at: user.registered_at,
                system: user.system,
            })
            .await?;

            if user.avatar.is_some() || user.banner.is_some() {
                let avatar_id_res = if let Some(avatar_id) = user.avatar {
                    Some(
                        self.state
                            .services()
                            .media
                            .load_remote_media(
                                user_id,
                                avatar_id,
                                Remote {
                                    origin_id: avatar_id.into(),
                                    hostname: hostname.clone(),
                                },
                            )
                            .await?
                            .id,
                    )
                } else {
                    None
                };

                let banner_id_res = if let Some(banner_id) = user.banner {
                    Some(
                        self.state
                            .services()
                            .media
                            .load_remote_media(
                                user_id,
                                banner_id,
                                Remote {
                                    origin_id: banner_id.into(),
                                    hostname: hostname.clone(),
                                },
                            )
                            .await?
                            .id,
                    )
                } else {
                    None
                };

                data.user_update(
                    user_id,
                    common::v1::types::UserPatch {
                        name: None,
                        description: None,
                        avatar: Some(avatar_id_res),
                        banner: Some(banner_id_res),
                    },
                )
                .await?;
            }
        }

        Ok(user)
    }
}
