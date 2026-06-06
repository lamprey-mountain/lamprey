use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use common::v1::types::notifications::bytes::NotificationBytes;
use common::v1::types::notifications::Notification;
use common::v1::types::UserId;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use lamprey_backend_data_postgres::PushData;
use p256::pkcs8::EncodePrivateKey;
use p256::SecretKey;
use reqwest::StatusCode;
use serde::Serialize;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tracing::{error, info};
use url::Url;

use crate::error::Error;
use crate::services::notifications::preferences::{calculate, NotificationAction};
use crate::{Result, ServerStateInner};

pub mod preferences;

pub struct ServiceNotifications {
    state: Arc<ServerStateInner>,
    vapid_keys: RwLock<Option<VapidKeys>>,
}

#[derive(Clone)]
struct VapidKeys {
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
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            vapid_keys: RwLock::new(None),
        }
    }

    /// Helper to fetch and parse VAPID keys from configuration
    async fn get_vapid_keys(&self) -> Result<VapidKeys> {
        let keys = self.vapid_keys.read().await;
        if let Some(keys) = &*keys {
            return Ok(keys.clone());
        }

        let mut data = self.state.data();
        let config_internal = data.config_get().await?;

        let (vapid_private, vapid_public) = match config_internal {
            Some(c) => (c.vapid_private_key, c.vapid_public_key),
            None => return Err(Error::Internal(format!("no internal config!"))),
        };

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
            public: vapid_private,
        };

        let mut self_keys = self.vapid_keys.write().await;
        *self_keys = Some(keys.clone());

        Ok(keys)
    }

    /// Send a notification to all of user's sessions via web push api
    ///
    /// pushes the notification to all sessions in parallel
    // NOTE: i may want to make this internal/private?
    pub async fn push(&self, user_id: UserId, mut payload: NotificationBytes) -> Result<()> {
        let mut data = self.state.data();
        let subscriptions = data.push_list_for_user(user_id).await?;
        let mut tasks = JoinSet::new();

        for sub in subscriptions {
            payload.set_session_id(sub.session_id);
            let state = Arc::clone(&self.state);
            tasks.spawn(Self::push_inner(state, sub, payload.to_bytes().into()));
        }

        while let Some(res) = tasks.join_next().await {
            res.map_err(|e| Error::Internal(format!("task join error: {}", e)))??;
        }

        Ok(())
    }

    /// send a notification to a session via web push api
    async fn push_inner(
        state: Arc<ServerStateInner>,
        sub: PushData,
        payload: bytes::Bytes,
    ) -> Result<()> {
        let vapid_keys = state.services().notifications.get_vapid_keys().await?;

        let p256dh_encoded = B64.encode(&sub.key_p256dh);
        let auth_encoded = B64.encode(&sub.key_auth);
        let p256dh = p256dh_encoded.as_bytes();
        let auth = auth_encoded.as_bytes();
        let ciphertext = ece::encrypt(p256dh, auth, &payload)
            .map_err(|e| Error::Internal(format!("encryption failed: {}", e)))?;
        let host = state
            .config
            .html_url // NOTE: why do i use html_url here?
            .host_str()
            .ok_or_else(|| Error::Internal("missing host in html_url".to_string()))?
            .to_string();

        let endpoint = Url::parse(&sub.endpoint)?;
        let claims = JwtClaims {
            aud: endpoint.origin().ascii_serialization(),
            exp: OffsetDateTime::now_utc().unix_timestamp() + 12 * 3600,
            // TODO: use something better?
            sub: format!("mailto:admin@{}", host),
        };

        let token = jsonwebtoken::encode(
            &Header::new(Algorithm::ES256),
            &claims,
            &vapid_keys.encoding,
        )?;

        // TODO: use http service
        // TODO: verify that headers/encoding are correct
        let client = reqwest::Client::new();
        let res = client
            .post(&sub.endpoint)
            .header("Content-Encoding", "aes128gcm")
            .header("TTL", "2419200")
            .header(
                "Authorization",
                format!("vapid t={token}, k={}", vapid_keys.public),
            )
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
                        let _ = state.data().push_delete(sub.session_id).await;
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

    pub fn start_background_tasks(&self) {
        tokio::spawn(Self::spawn_push_task(self.state.clone()));
    }

    async fn spawn_push_task(state: Arc<ServerStateInner>) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let mut data = state.data();
            let srv = state.services();

            let notifs = match data.notification_get_unpushed(50).await {
                Ok(notifs) => notifs,
                Err(e) => {
                    error!("failed to fetch unpushed notifications: {}", e);
                    continue;
                }
            };

            if notifs.is_empty() {
                continue;
            }

            let mut pushed_ids = Vec::new();
            for (user_id, notif) in notifs {
                let id = notif.id;
                if let Err(e) = srv.notifications.push(user_id, notif.into()).await {
                    error!("failed to push notification {id}: {e}");
                }
                pushed_ids.push(id);
            }

            if let Err(e) = data.notification_set_pushed(&pushed_ids).await {
                error!("failed to mark notifications as pushed: {}", e);
            }
        }
    }

    /// calculate the actions to take for a notification
    pub async fn calculate_actions(
        &self,
        user_id: UserId,
        notif: &Notification,
    ) -> Result<NotificationAction> {
        calculate(&self.state, user_id, notif).await
    }

    /// add a notification
    ///
    /// inserts into the database and executes any needed actions
    pub async fn create(&self, user_id: UserId, notification: Notification) -> Result<()> {
        let action = self.calculate_actions(user_id, &notification).await?;

        match (action.should_add_to_inbox(), action.should_push()) {
            (true, true) => {
                let mut data = self.state.acquire_data().await?;
                data.notification_add(user_id, notification).await?;
                data.commit().await?;
            }
            (true, false) => {
                let mut data = self.state.acquire_data().await?;
                data.notification_add(user_id, notification.clone()).await?;
                data.notification_set_pushed(&[notification.id]).await?;
                data.commit().await?;
            }
            (false, true) => {
                // NOTE: this branch currently isn't reachable
                // TODO: durable pushing without adding to inbox
                todo!()
            }
            (false, false) => {}
        }

        Ok(())
    }
}
