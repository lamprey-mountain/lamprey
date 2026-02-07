use std::sync::Arc;
use std::time::Duration;

use common::v1::types::{ChannelId, MessageId, NotificationId, UserId};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use p256::pkcs8::EncodePrivateKey;
use p256::SecretKey;
use reqwest::StatusCode;
use serde::Serialize;
use time::OffsetDateTime;
use tracing::{error, info};

use crate::error::Error;
use crate::{Result, ServerStateInner};

pub struct ServiceNotifications {
    state: Arc<ServerStateInner>,
}

/// payload sent via web push api
///
/// since the web push api has a pretty low payload size, generally around 2048
/// bytes, this is mostly a "wake up" notif. the client will fetch the full data
/// when receiving this.
#[derive(Debug, Serialize)]
pub struct NotificationPayload {
    pub id: NotificationId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
}

#[derive(Debug, Serialize)]
struct JwtClaims {
    aud: String,
    exp: i64,
    sub: String,
}

impl ServiceNotifications {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    /// Helper to fetch and parse VAPID keys from configuration
    async fn get_vapid_keys(&self) -> Result<Option<(EncodingKey, String)>> {
        let data = self.state.data();
        let config_internal = data.config_get().await?;

        let (vapid_private, vapid_public) = match config_internal {
            Some(c) => (c.vapid_private_key, c.vapid_public_key),
            None => return Ok(None),
        };

        let private_key_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            &vapid_private,
        )
        .map_err(|_| Error::Internal("invalid vapid key".to_string()))?;

        // Use p256 crate to convert raw private key bytes to PKCS#8 DER
        let secret_key = SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| Error::Internal(format!("invalid p256 secret key: {}", e)))?;

        let pkcs8_doc = secret_key
            .to_pkcs8_der()
            .map_err(|e| Error::Internal(format!("failed to convert to pkcs8: {}", e)))?;

        let encoding_key = EncodingKey::from_ec_der(pkcs8_doc.as_bytes());

        Ok(Some((encoding_key, vapid_public)))
    }

    fn generate_vapid_token(
        &self,
        endpoint: &str,
        encoding_key: &EncodingKey,
    ) -> Option<String> {
        let url = url::Url::parse(endpoint).ok()?;
        let origin = url.origin().ascii_serialization();
        
        let host = self
            .state
            .config
            .html_url
            .host_str()
            .unwrap_or("example.com")
            .to_string();

        let claims = JwtClaims {
            aud: origin,
            exp: OffsetDateTime::now_utc().unix_timestamp() + 12 * 3600,
            sub: format!("mailto:admin@{}", host),
        };

        match encode(&Header::new(Algorithm::ES256), &claims, encoding_key) {
            Ok(t) => Some(t),
            Err(e) => {
                error!("jwt encode failed: {}", e);
                None
            }
        }
    }

    /// send a notification to a user through the web push api
    pub async fn push(&self, user_id: UserId, payload: NotificationPayload) -> Result<()> {
        let data = self.state.data();
        let subscriptions = data.push_list_for_user(user_id).await?;

        if subscriptions.is_empty() {
            return Ok(());
        }

        let (encoding_key, vapid_public) = match self.get_vapid_keys().await? {
            Some(keys) => keys,
            None => return Ok(()),
        };

        let json_payload = serde_json::to_vec(&payload)?;

        for sub in subscriptions {
            let state = self.state.clone();
            let endpoint = sub.endpoint.clone();
            
            // Clone for the async task
            let encoding_key = encoding_key.clone();
            let vapid_public = vapid_public.clone();
            let json_payload = json_payload.clone();
            
            // Generate token before spawning or inside? Inside is safer for moved data.
            // But we need 'self' for generate_vapid_token if it uses self.state.config
            // So we'll pass the token generation logic or just do it inside.
            // To access `self` inside tokio::spawn we need to clone the arc state, 
            // but `generate_vapid_token` needs `self`.
            // Let's just create the token inside the loop *before* spawn or clone what's needed.
            // Actually, we can just extract the host string outside.
            
            let host = self
                .state
                .config
                .html_url
                .host_str()
                .unwrap_or("example.com")
                .to_string();

            let p256dh = base64::Engine::decode(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                &sub.key_p256dh,
            )
            .map_err(|_| Error::Internal("invalid p256dh".to_string()))?;
            let auth = base64::Engine::decode(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                &sub.key_auth,
            )
            .map_err(|_| Error::Internal("invalid auth".to_string()))?;

            let ciphertext = ece::encrypt(&p256dh, &auth, &json_payload)
                .map_err(|e| Error::Internal(format!("encryption failed: {}", e)))?;

            tokio::spawn(async move {
                let url_parsed = match url::Url::parse(&endpoint) {
                    Ok(u) => u,
                    Err(_) => return,
                };
                let origin = url_parsed.origin().ascii_serialization();

                let claims = JwtClaims {
                    aud: origin,
                    exp: OffsetDateTime::now_utc().unix_timestamp() + 12 * 3600,
                    sub: format!("mailto:admin@{}", host),
                };

                let token = match encode(&Header::new(Algorithm::ES256), &claims, &encoding_key) {
                    Ok(t) => t,
                    Err(e) => {
                        error!("jwt encode failed: {}", e);
                        return;
                    }
                };

                let client = reqwest::Client::new();
                let res = client
                    .post(&endpoint)
                    .header("Content-Encoding", "aes128gcm")
                    .header("TTL", "2419200")
                    .header("Authorization", format!("vapid t={token}, k={vapid_public}"))
                    .header("Crypto-Key", format!("p256ecdsa={vapid_public}"))
                    .body(ciphertext)
                    .send()
                    .await;

                match res {
                    Ok(res) => {
                        if !res.status().is_success() {
                            error!("failed to send push notification: status {}", res.status());
                            if res.status() == StatusCode::GONE
                                || res.status() == StatusCode::NOT_FOUND
                            {
                                info!("subscription gone, deleting");
                                let _ = state.data().push_delete(sub.session_id).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("failed to send push notification: {}", e);
                    }
                }
            });
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
            let data = state.data();
            let srv = state.services();

            match data.notification_get_unpushed(50).await {
                Ok(notifs) => {
                    if notifs.is_empty() {
                        continue;
                    }

                    let mut pushed_ids = Vec::new();
                    for (user_id, notif) in notifs {
                        let payload = NotificationPayload {
                            id: notif.id,
                            channel_id: notif.channel_id,
                            message_id: notif.message_id,
                        };

                        if let Err(e) = srv.notifications.push(user_id, payload).await {
                            error!("failed to push notification {}: {}", notif.id, e);
                        }
                        pushed_ids.push(notif.id);
                    }

                    if let Err(e) = data.notification_set_pushed(&pushed_ids).await {
                        error!("failed to mark notifications as pushed: {}", e);
                    }
                }
                Err(e) => {
                    error!("failed to fetch unpushed notifications: {}", e);
                }
            }
        }
    }
}
