use std::sync::Arc;
use std::time::Duration;

use common::v1::types::{ChannelId, MessageId, NotificationId, UserId};
use reqwest::StatusCode;
use serde::Serialize;
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

impl ServiceNotifications {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    /// send a notification to a user through the web push api
    pub async fn push(&self, user_id: UserId, payload: NotificationPayload) -> Result<()> {
        let data = self.state.data();
        let subscriptions = data.push_list_for_user(user_id).await?;

        if subscriptions.is_empty() {
            return Ok(());
        }

        let json_payload = serde_json::to_vec(&payload)?;

        for sub in subscriptions {
            let state = self.state.clone();
            let endpoint = sub.endpoint.clone();
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
                let client = reqwest::Client::new();
                let res = client
                    .post(&endpoint)
                    .header("Content-Encoding", "aes128gcm")
                    .header("TTL", "2419200")
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
