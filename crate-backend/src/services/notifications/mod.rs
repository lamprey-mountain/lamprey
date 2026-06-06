use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use common::v1::types::notifications::bytes::NotificationBytes;
use common::v1::types::notifications::{Notification, NotificationType};
use common::v1::types::util::Time;
use common::v1::types::{ChannelId, Message, MessageId, NotificationId, SessionId, UserId};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use lamprey_backend_data_postgres::{Channel, PushData};
use p256::pkcs8::EncodePrivateKey;
use p256::SecretKey;
use reqwest::StatusCode;
use serde::Serialize;
use time::OffsetDateTime;
use tokio::task::JoinSet;
use tracing::{error, info, warn};
use url::Url;

use crate::error::Error;
use crate::services::notifications::preferences::{
    NotificationAction, NotificationActionCalculator,
};
use crate::{Result, ServerStateInner};

pub mod preferences;

pub struct ServiceNotifications {
    state: Arc<ServerStateInner>,
}

// TODO: review llm generated code (vapid logic)

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
    pub session_id: SessionId,
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
        let mut data = self.state.data();
        let config_internal = data.config_get().await?;

        let (vapid_private, vapid_public) = match config_internal {
            Some(c) => (c.vapid_private_key, c.vapid_public_key),
            None => return Ok(None),
        };

        let encoded_private = B64.encode(&vapid_private);
        let private_key_bytes = encoded_private.as_bytes();

        // Use p256 crate to convert raw private key bytes to PKCS#8 DER
        let secret_key = SecretKey::from_slice(private_key_bytes)
            .map_err(|e| Error::Internal(format!("invalid p256 secret key: {}", e)))?;

        let pkcs8_doc = secret_key
            .to_pkcs8_der()
            .map_err(|e| Error::Internal(format!("failed to convert to pkcs8: {}", e)))?;

        let encoding_key = EncodingKey::from_ec_der(pkcs8_doc.as_bytes());

        Ok(Some((encoding_key, vapid_public)))
    }

    /// Send a notification to all of user's sessions via web push api
    ///
    /// pushes the notification to all sessions in parallel
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
    pub async fn push_inner(
        state: Arc<ServerStateInner>,
        sub: PushData,
        payload: bytes::Bytes,
    ) -> Result<()> {
        let (encoding_key, vapid_public) =
            match state.services().notifications.get_vapid_keys().await? {
                Some(keys) => keys,
                None => {
                    warn!("vapid keys not found");
                    return Ok(());
                }
            };

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

        let token = jsonwebtoken::encode(&Header::new(Algorithm::ES256), &claims, &encoding_key)?;

        // TODO: use http service
        let client = reqwest::Client::new();
        let res = client
            .post(&sub.endpoint)
            .header("Content-Encoding", "aes128gcm")
            .header("TTL", "2419200")
            .header(
                "Authorization",
                format!("vapid t={token}, k={vapid_public}"),
            )
            .header("Crypto-Key", format!("p256ecdsa={vapid_public}"))
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

    /// get a notification calculator for a user
    pub fn calculator(
        &self,
        user_id: UserId,
        notif: &Notification,
    ) -> NotificationActionCalculator {
        NotificationActionCalculator::new(self.state.clone(), user_id, notif.clone())
    }

    /// create notifications for a new message
    // TODO: redo
    pub async fn dispatch_message_creation(
        &self,
        message: &Message,
        chan: &Channel,
        author_id: UserId,
    ) {
        let mentions = &message.latest_version.mentions;
        let mut notified_users = HashSet::new();
        let is_thread = chan.is_thread();

        // 1. Resolve all users that need to be notified
        let mut users_to_notify = Vec::new();

        // Add directly mentioned users
        users_to_notify.extend(mentions.users.iter().map(|u| u.id));

        // Add role mention users
        if chan.room_id.is_some() {
            for role in &mentions.roles {
                // TODO: use room cache/service for member list
                if let Ok(members) = self
                    .state
                    .data()
                    .role_member_list(role.id, Default::default())
                    .await
                {
                    users_to_notify.extend(members.items.into_iter().map(|m| m.user_id));
                }
            }
        }

        // Add @everyone users
        if mentions.everyone {
            if is_thread {
                if let Ok(members) = self.state.data().thread_member_list_all(chan.id).await {
                    users_to_notify.extend(members.into_iter().map(|m| m.user_id));
                }
            } else if let Some(room_id) = chan.room_id {
                if let Ok(members) = self.state.data().room_member_list_all(room_id).await {
                    users_to_notify.extend(members.into_iter().map(|m| m.user_id));
                }
            }
        }

        // 2. Process Notifications
        for user_id in users_to_notify {
            if user_id == author_id || !notified_users.insert(user_id) {
                continue; // Skip author and duplicates
            }

            // Optional: Handle thread auto-join logic here

            // Increment mentions
            // PERF: bulk increment mentions count
            let _ = self
                .state
                .data()
                .unread_increment_mentions(
                    user_id,
                    chan.id,
                    message.id,
                    message.latest_version.version_id,
                    1,
                )
                .await;

            // Generate Inbox Notification
            let room_id_opt = self
                .state
                .services()
                .channels
                .get(chan.id, Some(user_id))
                .await
                .ok()
                .and_then(|c| c.room_id);
            let notification = Notification {
                id: NotificationId::new(),
                ty: NotificationType::Message {
                    room_id: room_id_opt,
                    channel_id: chan.id,
                    message_id: message.id,
                },
                added_at: Time::now_utc(),
                read_at: None,
                note: None,
            };

            let action = self
                .calculator(user_id, &notification)
                .action()
                .await
                .unwrap_or(NotificationAction::Push);

            if action.should_add_to_inbox() {
                let _ = self
                    .state
                    .data()
                    .notification_add(user_id, notification)
                    .await;
            }
        }
    }

    // TODO: add?
    // pub async fn create(&self, user_id: UserId, notification: Notification) {
    //     let action = srv
    //         .notifications
    //         .calculator(user_id, &notification)
    //         .action()
    //         .await
    //         .unwrap_or(NotificationAction::Skip);

    //     if action.should_add_to_inbox() {
    //         todo!()
    //         // data.notification_add(user_id, notification).await?;
    //     }
    // }
}
