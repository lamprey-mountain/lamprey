use crate::error::{Error, Result};
use crate::services::federation::ServiceFederation;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use common::v1::types::federation::consts::{
    EXPIRED_KEY_RETENTION, KEY_ROTATION_WINDOW,
};
use common::v1::types::federation::signing::ServerKeySecret;
use common::v1::types::federation::ServerKeyAlgorithm;
use common::v1::types::util::Time;
use ed25519_dalek::{SigningKey, VerifyingKey};
use lamprey_backend_core::config::ServerKeyInternal;

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

fn from_internal(key: &ServerKeyInternal) -> Result<ServerKeySecret> {
    if key.alg != ServerKeyAlgorithm::Ed25519 {
        return Err(Error::BadStatic("unsupported key algorithm"));
    }

    let pubkey_bytes = Engine::decode(&URL_SAFE_NO_PAD, &key.pubkey)
        .map_err(|_| Error::BadStatic("invalid pubkey encoding"))?;

    let privkey_bytes = Engine::decode(&URL_SAFE_NO_PAD, &key.privkey)
        .map_err(|_| Error::BadStatic("invalid privkey encoding"))?;

    let pubkey: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| Error::BadStatic("invalid pubkey length"))?;

    let privkey: [u8; 64] = privkey_bytes
        .try_into()
        .map_err(|_| Error::BadStatic("invalid privkey length"))?;

    let signing_key = SigningKey::from_keypair_bytes(&privkey)
        .map_err(|_| Error::BadStatic("invalid privkey key"))?;
    let pubkey_parsed =
        VerifyingKey::from_bytes(&pubkey).map_err(|_| Error::BadStatic("invalid public key"))?;

    Ok(ServerKeySecret {
        pubkey: pubkey_parsed,
        signing_key,
        expires_at: key.expires_at,
    })
}

fn to_internal(key: &ServerKeySecret) -> ServerKeyInternal {
    let pubkey_encoded = Engine::encode(&URL_SAFE_NO_PAD, key.pubkey.to_bytes());
    let privkey_encoded = Engine::encode(&URL_SAFE_NO_PAD, key.signing_key.to_keypair_bytes());

    ServerKeyInternal {
        alg: ServerKeyAlgorithm::Ed25519,
        pubkey: pubkey_encoded,
        privkey: privkey_encoded,
        expires_at: key.expires_at,
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

impl ServiceFederation {
    /// load local signing keys from config
    pub async fn load_local_keys(&self) -> Result<()> {
        let config = self
            .state
            .data()
            .config_get()
            .await?
            .ok_or_else(|| Error::Internal("internal config not initialized".to_string()))?;

        let local_keys: Vec<ServerKeySecret> = config
            .federation_keys
            .iter()
            .filter_map(|k| from_internal(k).ok())
            .collect();

        *self.local_keys.write().await = local_keys;
        Ok(())
    }

    /// regenerate local signing keys
    ///
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
            let new_key = ServerKeySecret::generate_new();
            local_keys.push(new_key);

            // TODO: use admin or something as the sole reader/writer for internal config
            let mut config =
                self.state.data().config_get().await?.ok_or_else(|| {
                    Error::Internal("internal config not initialized".to_string())
                })?;

            config.federation_keys = local_keys.iter().map(to_internal).collect();
            self.state.data().config_put(config).await?;
        }

        Ok(())
    }

    /// get the current valid local signing keys
    pub async fn get_local_keys(&self) -> Vec<ServerKeySecret> {
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
    pub async fn get_all_local_keys(&self) -> Vec<ServerKeySecret> {
        self.local_keys.read().await.clone()
    }
}
