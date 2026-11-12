use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use common::v1::types::notifications::bytes::NotificationBytes;
use common::v1::types::{SessionId, UserId};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use lamprey_backend_data_postgres::PushData;
use p256::SecretKey;
use p256::pkcs8::EncodePrivateKey;
use reqwest::StatusCode;
use serde::Serialize;
use time::OffsetDateTime;
use tokio::task::JoinSet;
use tracing::{error, info};
use url::Url;

use crate::prelude::*;
use crate::services::notifications::ServiceNotifications;

#[derive(Clone)]
pub struct VapidKeys {
    encoding: EncodingKey,
    public: String,
}

// TODO: use actual struct
#[derive(Debug, Serialize)]
struct JwtClaims {
    aud: String,
    exp: i64,
    sub: String,
}

impl ServiceNotifications {
    /// Helper to fetch and parse VAPID keys from configuration
    async fn get_vapid_keys(&self) -> Result<VapidKeys> {
        let keys = self.vapid_keys.read().await;
        if let Some(keys) = &*keys {
            return Ok(keys.clone());
        }

        let srv = self.globals.services();
        let c = srv.config.internal_get().await?;

        let (vapid_private, vapid_public) = (c.vapid_private_key, c.vapid_public_key);

        let decoded_private = B64
            .decode(&vapid_private)
            .map_err(|e| Error::Internal(format!("invalid vapid private key: {}", e)))?;

        // Use p256 crate to convert raw private key bytes to PKCS#8 DER
        let secret_key = SecretKey::from_slice(&decoded_private)
            .map_err(|e| Error::Internal(format!("invalid p256 secret key: {}", e)))?;

        let pkcs8_doc = secret_key
            .to_pkcs8_der()
            .map_err(|e| Error::Internal(format!("failed to convert to pkcs8: {}", e)))?;

        let encoding_key = EncodingKey::from_ec_der(pkcs8_doc.as_bytes());

        let keys = VapidKeys {
            encoding: encoding_key,
            public: vapid_public,
        };

        let mut self_keys = self.vapid_keys.write().await;
        *self_keys = Some(keys.clone());

        Ok(keys)
    }

    /// Send a notification to all of user's sessions via web push api
    ///
    /// pushes the notification to all sessions in parallel
    // NOTE: i may want to make this internal/private?
    #[tracing::instrument(
        name = "notifications.push",
        skip_all,
        fields(user_id = %user_id),
        err,
    )]
    pub async fn push(&self, user_id: UserId, mut payload: NotificationBytes) -> Result<()> {
        let mut data = self.globals.begin_read().await?;
        let subscriptions = data.push_list_for_user(user_id).await?;
        let mut tasks = JoinSet::new();

        for sub in subscriptions {
            payload.set_session_id(sub.session_id);
            let state = self.globals.clone();
            tasks.spawn(Self::push_inner(state, sub, payload.to_bytes().into()));
        }

        while let Some(res) = tasks.join_next().await {
            res.map_err(|e| Error::Internal(format!("task join error: {}", e)))??;
        }

        Ok(())
    }

    /// unregister a session's subscription from its push service and delete it
    ///
    /// best effort: failures to reach the push service are logged, not returned
    #[tracing::instrument(name = "notifications.push.unregister", skip_all, fields(session_id = %session_id))]
    pub async fn push_unregister_session(&self, session_id: SessionId) -> Result<()> {
        let mut data = self.globals.begin().await?;
        let subscription = match data.push_get(session_id).await {
            Ok(sub) => Some(sub),
            Err(Error::NotFound) => None,
            Err(e) => return Err(e),
        };
        data.push_delete(session_id).await?;
        data.commit().await?;

        if let Some(sub) = subscription {
            tokio::spawn(Self::push_unregister_inner(self.globals.clone(), sub));
        }

        Ok(())
    }

    /// unregister all of a user's subscriptions from their push services and delete them
    ///
    /// best effort: failures to reach the push service are logged, not returned
    #[tracing::instrument(name = "notifications.push.unregister_all", skip_all, fields(user_id = %user_id))]
    pub async fn push_unregister_user(&self, user_id: UserId) -> Result<()> {
        let mut data = self.globals.begin().await?;
        let subscriptions = data.push_list_for_user(user_id).await?;
        data.push_delete_for_user(user_id).await?;
        data.commit().await?;

        for sub in subscriptions {
            tokio::spawn(Self::push_unregister_inner(self.globals.clone(), sub));
        }

        Ok(())
    }

    /// send a notification to a session via web push api
    #[tracing::instrument(
        name = "notifications.push.inner",
        skip_all,
        fields(push.user_id = %sub.user_id, push.session_id = %sub.session_id),
        err,
    )]
    async fn push_inner(state: Globals, sub: PushData, payload: Bytes) -> Result<()> {
        let vapid_keys = state.services().notifications.get_vapid_keys().await?;

        let p256dh = B64
            .decode(&sub.key_p256dh)
            .map_err(|e| Error::Internal(format!("invalid p256dh key: {}", e)))?;
        let auth = B64
            .decode(&sub.key_auth)
            .map_err(|e| Error::Internal(format!("invalid auth key: {}", e)))?;
        let ciphertext = ece::encrypt(&p256dh, &auth, &payload)
            .map_err(|e| Error::Internal(format!("encryption failed: {}", e)))?;

        let endpoint = Url::parse(&sub.endpoint)?;
        let authorization = state
            .services()
            .notifications
            .vapid_authorization(&endpoint)
            .await?;

        // TODO: use http service
        // TODO: verify that headers/encoding are correct
        let client = reqwest::Client::new();
        let res = client
            .post(&sub.endpoint)
            .header("Content-Encoding", "aes128gcm")
            .header("TTL", "2419200")
            .header("Authorization", authorization)
            .header("Crypto-Key", format!("p256ecdsa={}", vapid_keys.public))
            .body(ciphertext)
            .send()
            .await;

        match res {
            Ok(res) => {
                if !res.status().is_success() {
                    error!("failed to send push notification: status {}", res.status());
                    if res.status() == StatusCode::GONE || res.status() == StatusCode::NOT_FOUND {
                        info!("subscription gone, deleting");
                        let mut data = state.begin().await?;
                        let _ = data.push_delete(sub.session_id).await;
                        let _ = data.commit().await?;
                        // TODO: better error handling? its fine if we can't delete it immediately though
                    }
                }
            }
            Err(e) => {
                error!("failed to send push notification: {e}");
                return Err(e.into());
            }
        }

        Ok(())
    }

    /// unsubscribe a subscription from its push service
    async fn push_unregister_inner(state: Globals, sub: PushData) {
        if let Err(e) = Self::push_unregister_request(&state, &sub).await {
            error!(
                "failed to unregister push subscription for session {}: {e}",
                sub.session_id
            );
        }
    }

    async fn push_unregister_request(state: &Globals, sub: &PushData) -> Result<()> {
        let endpoint = Url::parse(&sub.endpoint)?;
        let authorization = state
            .services()
            .notifications
            .vapid_authorization(&endpoint)
            .await?;

        // TODO: use http service
        let res = reqwest::Client::new()
            .delete(endpoint.as_str())
            .header("Authorization", authorization)
            .send()
            .await?;

        match res.status() {
            status if status.is_success() => Ok(()),
            // subscription is already gone
            StatusCode::GONE | StatusCode::NOT_FOUND => Ok(()),
            status => Err(Error::Internal(format!(
                "unexpected status unregistering push subscription: {status}"
            ))),
        }
    }

    /// build a vapid `Authorization` header value for a push endpoint
    async fn vapid_authorization(&self, endpoint: &Url) -> Result<String> {
        let keys = self.get_vapid_keys().await?;
        let host = self
            .globals
            .config()
            .html_url // NOTE: why do i use html_url here?
            .host_str()
            .ok_or_else(|| Error::Internal("missing host in html_url".to_string()))?
            .to_string();

        let claims = JwtClaims {
            aud: endpoint.origin().ascii_serialization(),
            exp: OffsetDateTime::now_utc().unix_timestamp() + 12 * 3600,
            // TODO: use something better?
            sub: format!("mailto:admin@{}", host),
        };

        let token = jsonwebtoken::encode(&Header::new(Algorithm::ES256), &claims, &keys.encoding)?;

        Ok(format!("vapid t={token}, k={}", keys.public))
    }

    pub(super) async fn spawn_push_task(state: Globals) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let mut data = match state.begin().await {
                Ok(tx) => tx,
                Err(e) => {
                    error!("failed to begin transaction: {}", e);
                    continue;
                }
            };

            let srv = state.services();

            let notifs = match data.notification_get_unpushed(50).await {
                Ok(notifs) => notifs,
                Err(e) => {
                    error!("failed to fetch unpushed notifications: {}", e);
                    continue;
                }
            };

            if notifs.is_empty() {
                let _ = data.rollback().await;
                continue;
            }

            let mut pushed_ids = Vec::new();
            for (user_id, notif) in notifs {
                let id = notif.id;
                match srv.notifications.push(user_id, notif.into()).await {
                    Ok(()) => pushed_ids.push(id),
                    Err(e) => error!("failed to push notification {id}: {e}"),
                }
            }

            if let Err(e) = data.notification_set_pushed(&pushed_ids).await {
                error!("failed to mark notifications as pushed: {}", e);
            }

            if let Err(e) = data.commit().await {
                error!("failed to commit transaction: {}", e);
            }
        }
    }
}
