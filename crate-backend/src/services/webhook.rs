use std::sync::Arc;
use std::time::Duration;

use common::v1::types::audit_logs::AuditLogEntryType;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::sync::MessageSync;
use common::v1::types::util::Changes;
use common::v1::types::webhook::{Webhook, WebhookCreate, WebhookUpdate};
use common::v1::types::{
    ChannelId, PaginationQuery, PaginationResponse, Permission, RoomId, WebhookId,
};
use moka::future::Cache;
use validator::Validate;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::ServerStateInner;

pub struct ServiceWebhooks {
    state: Arc<ServerStateInner>,
    idempotency_keys: Cache<String, Webhook>,
}

impl ServiceWebhooks {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            idempotency_keys: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    pub async fn create(
        &self,
        channel_id: ChannelId,
        auth: &Auth,
        json: WebhookCreate,
        nonce: Option<String>,
    ) -> Result<Webhook> {
        if let Some(n) = &nonce {
            self.idempotency_keys
                .try_get_with(
                    n.clone(),
                    self.create_inner(channel_id, auth, json, nonce.clone()),
                )
                .await
                .map_err(|err| err.fake_clone())
        } else {
            self.create_inner(channel_id, auth, json, nonce).await
        }
    }

    async fn create_inner(
        &self,
        channel_id: ChannelId,
        auth: &Auth,
        json: WebhookCreate,
        nonce: Option<String>,
    ) -> Result<Webhook> {
        json.validate()?;
        let data = self.state.data();
        let srv = self.state.services();

        let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
        let chan = srv.channels.get(channel_id, None).await?;
        let room_id = chan
            .room_id
            .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

        perms.ensure(Permission::IntegrationsManage)?;

        chan.ensure_has_text()?;

        let webhook = data
            .webhook_create(channel_id, auth.user.id, json.clone())
            .await?;

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::WebhookCreate {
            webhook_id: webhook.id,
            changes: Changes::new()
                .add("name", &webhook.name)
                .add("channel_id", &webhook.channel_id)
                .build(),
        })
        .await?;

        let sync_msg = MessageSync::WebhookCreate {
            webhook: webhook.clone(),
        };
        self.state
            .broadcast_room_with_nonce(room_id, auth.user.id, nonce.as_deref(), sync_msg)
            .await?;

        Ok(webhook)
    }

    pub async fn get(&self, webhook_id: WebhookId) -> Result<Webhook> {
        self.state.data().webhook_get(webhook_id).await
    }

    pub async fn update(
        &self,
        webhook_id: WebhookId,
        auth: &Auth,
        json: WebhookUpdate,
    ) -> Result<Webhook> {
        let data = self.state.data();
        let webhook = data.webhook_get(webhook_id).await?;

        let chan = self
            .state
            .services()
            .channels
            .get(webhook.channel_id, None)
            .await?;
        let room_id = chan
            .room_id
            .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

        let perms = self
            .state
            .services()
            .perms
            .for_channel(auth.user.id, webhook.channel_id)
            .await?;
        perms.ensure(Permission::IntegrationsManage)?;

        let webhook_before = webhook.clone();
        let webhook = data.webhook_update(webhook_id, json).await?;

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::WebhookUpdate {
            webhook_id,
            changes: Changes::new()
                .change("name", &webhook_before.name, &webhook.name)
                .change(
                    "channel_id",
                    &webhook_before.channel_id,
                    &webhook.channel_id,
                )
                .build(),
        })
        .await?;

        let sync_msg = MessageSync::WebhookUpdate {
            webhook: webhook.clone(),
        };
        self.state
            .broadcast_room(room_id, auth.user.id, sync_msg)
            .await?;

        Ok(webhook)
    }

    pub async fn delete(&self, webhook_id: WebhookId, auth: &Auth) -> Result<()> {
        let data = self.state.data();
        let webhook = data.webhook_get(webhook_id).await?;

        let chan = self
            .state
            .services()
            .channels
            .get(webhook.channel_id, None)
            .await?;
        let room_id = chan
            .room_id
            .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

        let perms = self
            .state
            .services()
            .perms
            .for_channel(auth.user.id, webhook.channel_id)
            .await?;
        perms.ensure(Permission::IntegrationsManage)?;

        data.webhook_delete(webhook_id).await?;

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::WebhookDelete {
            webhook_id,
            changes: Changes::new()
                .remove("name", &webhook.name)
                .remove("channel_id", &webhook.channel_id)
                .build(),
        })
        .await?;

        let sync_msg = MessageSync::WebhookDelete {
            channel_id: webhook.channel_id,
            webhook_id,
            room_id: Some(room_id),
        };
        self.state
            .broadcast_room(room_id, auth.user.id, sync_msg)
            .await?;

        Ok(())
    }

    pub async fn list_channel(
        &self,
        channel_id: ChannelId,
        pagination: PaginationQuery<WebhookId>,
    ) -> Result<PaginationResponse<Webhook>> {
        self.state
            .data()
            .webhook_list_channel(channel_id, pagination)
            .await
    }

    pub async fn list_room(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<WebhookId>,
    ) -> Result<PaginationResponse<Webhook>> {
        self.state
            .data()
            .webhook_list_room(room_id, pagination)
            .await
    }
}
