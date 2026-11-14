use std::time::Duration;

use common::v1::types::audit_logs::{AuditLogChange, AuditLogEntryType};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::sync::MessageSync;
use common::v1::types::util::Changes;
use common::v1::types::webhook::{Webhook, WebhookCreate, WebhookUpdate};
use common::v1::types::{
    ChannelId, Message, MessageCreate, PaginationQuery, PaginationResponse, Permission, RoomId,
    UserId, WebhookId,
};
use kerosene_core::types::auth::{Auth5, Auth5Ext};
use moka::future::Cache;
use serde_json::Value;
use validator::Validate;

use crate::prelude::*;

pub struct ServiceWebhooks {
    state: Globals,
    idempotency_keys: Cache<String, Webhook>,
}

impl ServiceWebhooks {
    pub fn new(state: Globals) -> Self {
        Self {
            state,
            idempotency_keys: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    pub async fn create<A: Auth5>(
        &self,
        channel_id: ChannelId,
        auth: &mut A,
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

    async fn create_inner<A: Auth5>(
        &self,
        channel_id: ChannelId,
        auth: &mut A,
        json: WebhookCreate,
        _nonce: Option<String>,
    ) -> Result<Webhook> {
        json.validate()?;
        let mut data = self.state.begin().await?;
        let srv = self.state.services();

        let user = auth.ensure_user()?;
        let user_id = user.id;
        let perms = srv.perms.for_channel(user_id, channel_id).await?;
        let chan = srv.channels.get(channel_id, None).await?;
        let room_id = chan
            .room_id
            .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

        perms.ensure(Permission::IntegrationsManage)?;

        chan.ensure_has_text()?;

        let webhook = data
            .webhook_create(channel_id, user_id, json.clone())
            .await?;
        data.commit().await?;

        auth.set_room_id(room_id);
        auth.al_push(AuditLogEntryType::WebhookCreate {
            webhook_id: webhook.id,
            changes: Changes::new()
                .add("name", &webhook.name)
                .add("avatar", &webhook.avatar)
                .add("channel_id", &webhook.channel_id)
                .build(),
        });

        let sync_msg = MessageSync::WebhookCreate {
            webhook: webhook.clone(),
        };
        self.state
            .messaging()
            .broadcast_room(room_id, sync_msg)
            .await?;

        Ok(webhook)
    }

    pub async fn get(&self, webhook_id: WebhookId) -> Result<Webhook> {
        self.state.begin_read().await?.webhook_get(webhook_id).await
    }

    pub async fn get_with_token(&self, webhook_id: WebhookId, token: &str) -> Result<Webhook> {
        self.state
            .begin_read()
            .await?
            .webhook_get_with_token(webhook_id, token)
            .await
    }

    pub async fn execute(
        &self,
        webhook_id: WebhookId,
        token: &str,
        json: MessageCreate,
    ) -> Result<Message> {
        let webhook = self.get_with_token(webhook_id, token).await?;

        let srv = self.state.services();
        let chan = srv.channels.get(webhook.channel_id, None).await?;
        chan.ensure_has_text()?;

        let author_id: UserId = (*webhook.id).into();
        srv.messages
            .create_as_webhook(webhook.channel_id, author_id, json, None)
            .await
    }

    pub async fn update<A: Auth5>(
        &self,
        webhook_id: WebhookId,
        auth: &mut A,
        json: WebhookUpdate,
    ) -> Result<Webhook> {
        json.validate()?;
        let rotate_token = json.rotate_token;

        let mut data = self.state.begin().await?;
        let webhook = data.webhook_get(webhook_id).await?;

        let srv = self.state.services();
        let chan = srv.channels.get(webhook.channel_id, None).await?;
        let room_id = chan
            .room_id
            .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

        let user = auth.ensure_user()?;
        let user_id = user.id;
        let perms = srv.perms.for_channel(user_id, webhook.channel_id).await?;
        perms.ensure(Permission::IntegrationsManage)?;

        let webhook_before = webhook.clone();
        let webhook = data.webhook_update(webhook_id, json).await?;
        data.commit().await?;

        let mut changes = Changes::new()
            .change("name", &webhook_before.name, &webhook.name)
            .change("avatar", &webhook_before.avatar, &webhook.avatar)
            .change(
                "channel_id",
                &webhook_before.channel_id,
                &webhook.channel_id,
            )
            .build();
        if rotate_token {
            changes.push(AuditLogChange {
                key: "token".to_string(),
                old: Value::Null,
                new: Value::Null,
            });
        }

        auth.set_room_id(room_id);
        if !changes.is_empty() {
            auth.al_push(AuditLogEntryType::WebhookUpdate {
                webhook_id,
                changes,
            });
        }

        let sync_msg = MessageSync::WebhookUpdate {
            webhook: webhook.clone(),
        };
        self.state
            .messaging()
            .broadcast_room(room_id, sync_msg)
            .await?;
        self.broadcast_user_update(webhook_id).await?;

        Ok(webhook)
    }

    pub async fn update_with_token(
        &self,
        webhook_id: WebhookId,
        token: &str,
        json: WebhookUpdate,
    ) -> Result<Webhook> {
        let json = WebhookUpdate {
            channel_id: None,
            ..json
        };
        json.validate()?;

        let mut data = self.state.begin().await?;
        let webhook = data
            .webhook_update_with_token(webhook_id, token, json)
            .await?;
        data.commit().await?;

        if let Some(room_id) = webhook.room_id {
            let sync_msg = MessageSync::WebhookUpdate {
                webhook: webhook.clone(),
            };
            self.state
                .messaging()
                .broadcast_room(room_id, sync_msg)
                .await?;
        }
        self.broadcast_user_update(webhook_id).await?;

        Ok(webhook)
    }

    pub async fn delete<A: Auth5>(&self, webhook_id: WebhookId, auth: &mut A) -> Result<()> {
        let mut data = self.state.begin().await?;
        let webhook = data.webhook_get(webhook_id).await?;

        let srv = self.state.services();
        let chan = srv.channels.get(webhook.channel_id, None).await?;
        let room_id = chan
            .room_id
            .ok_or_else(|| ApiError::from_code(ErrorCode::ChannelNotInRoom))?;

        let user = auth.ensure_user()?;
        let user_id = user.id;
        let perms = srv.perms.for_channel(user_id, webhook.channel_id).await?;
        perms.ensure(Permission::IntegrationsManage)?;

        data.webhook_delete(webhook_id).await?;
        data.commit().await?;

        auth.set_room_id(room_id);
        auth.al_push(AuditLogEntryType::WebhookDelete {
            webhook_id,
            changes: Changes::new()
                .remove("name", &webhook.name)
                .remove("channel_id", &webhook.channel_id)
                .build(),
        });

        let sync_msg = MessageSync::WebhookDelete {
            channel_id: webhook.channel_id,
            webhook_id,
            room_id: Some(room_id),
        };
        self.state
            .messaging()
            .broadcast_room(room_id, sync_msg)
            .await?;

        Ok(())
    }

    pub async fn delete_with_token(&self, webhook_id: WebhookId, token: &str) -> Result<()> {
        let mut data = self.state.begin().await?;
        data.webhook_delete_with_token(webhook_id, token).await?;
        data.commit().await
    }

    /// invalidate the webhook's user and tell everyone about it
    async fn broadcast_user_update(&self, webhook_id: WebhookId) -> Result<()> {
        let user_id: UserId = (*webhook_id).into();
        let srv = self.state.services();
        srv.users.invalidate(user_id).await;
        let user = srv.users.get(user_id, None).await?;

        self.state
            .messaging()
            .broadcast_global(MessageSync::UserUpdate { user })
            .await
    }

    pub async fn list_channel(
        &self,
        channel_id: ChannelId,
        pagination: PaginationQuery<WebhookId>,
    ) -> Result<PaginationResponse<Webhook>> {
        self.state
            .begin_read()
            .await?
            .webhook_list_channel(channel_id, pagination)
            .await
    }

    pub async fn list_room(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<WebhookId>,
    ) -> Result<PaginationResponse<Webhook>> {
        self.state
            .begin_read()
            .await?
            .webhook_list_room(room_id, pagination)
            .await
    }
}
