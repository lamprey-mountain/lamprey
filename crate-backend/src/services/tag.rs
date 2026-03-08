use std::sync::Arc;
use std::time::Duration;

use common::v1::types::audit_logs::AuditLogEntryType;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::sync::MessageSync;
use common::v1::types::tag::{Tag, TagCreate, TagPatch};
use common::v1::types::util::Changes;
use common::v1::types::{ChannelId, PaginationQuery, PaginationResponse, Permission, TagId};
use moka::future::Cache;
use validator::Validate;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::{Error, ServerStateInner};

pub struct ServiceTags {
    state: Arc<ServerStateInner>,
    idempotency_keys: Cache<String, Tag>,
}

impl ServiceTags {
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
        create: TagCreate,
        nonce: Option<String>,
    ) -> Result<Tag> {
        if let Some(n) = &nonce {
            self.idempotency_keys
                .try_get_with(
                    n.clone(),
                    self.create_inner(channel_id, auth, create, nonce.clone()),
                )
                .await
                .map_err(|err| err.fake_clone())
        } else {
            self.create_inner(channel_id, auth, create, nonce).await
        }
    }

    async fn create_inner(
        &self,
        channel_id: ChannelId,
        auth: &Auth,
        create: TagCreate,
        nonce: Option<String>,
    ) -> Result<Tag> {
        create.validate()?;
        let data = self.state.data();
        let srv = self.state.services();

        let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
        perms.ensure(Permission::TagManage)?;

        let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
        if !channel.ty.has_tags() {
            return Err(ApiError::from_code(ErrorCode::ChannelDoesNotSupportTags).into());
        }

        let tag = data.tag_create(channel_id, create.clone()).await?;

        if let Some(room_id) = channel.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::TagCreate {
                channel_id,
                tag_id: tag.id,
                changes: Changes::new()
                    .add("name", &tag.name)
                    .add("description", &tag.description)
                    .add("color", &tag.color)
                    .add("restricted", &tag.restricted)
                    .build(),
            })
            .await?;
        }

        let sync_msg = MessageSync::TagCreate { tag: tag.clone() };
        self.state
            .broadcast_channel_with_nonce(channel_id, auth.user.id, nonce.as_deref(), sync_msg)
            .await?;

        Ok(tag)
    }

    pub async fn get(&self, tag_id: TagId) -> Result<Tag> {
        self.state.data().tag_get(tag_id).await
    }

    pub async fn update(
        &self,
        channel_id: ChannelId,
        tag_id: TagId,
        auth: &Auth,
        patch: TagPatch,
    ) -> Result<Tag> {
        let data = self.state.data();
        let srv = self.state.services();

        let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
        perms.ensure(Permission::TagManage)?;

        let tag_channel_id = data.tag_get_forum_id(tag_id).await?;
        if channel_id != tag_channel_id {
            return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownTag)));
        }

        let tag_old = data.tag_get(tag_id).await?;
        let tag = data.tag_update(tag_id, patch).await?;

        let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
        if let Some(room_id) = channel.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::TagUpdate {
                channel_id,
                tag_id,
                changes: Changes::new()
                    .change("name", &tag_old.name, &tag.name)
                    .change("description", &tag_old.description, &tag.description)
                    .change("color", &tag_old.color, &tag.color)
                    .change("archived", &tag_old.archived, &tag.archived)
                    .change("restricted", &tag_old.restricted, &tag.restricted)
                    .build(),
            })
            .await?;
        }

        let sync_msg = MessageSync::TagUpdate { tag: tag.clone() };
        self.state
            .broadcast_channel(channel_id, auth.user.id, sync_msg)
            .await?;

        Ok(tag)
    }

    pub async fn delete(
        &self,
        channel_id: ChannelId,
        tag_id: TagId,
        auth: &Auth,
        force: bool,
    ) -> Result<()> {
        let data = self.state.data();
        let srv = self.state.services();

        let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
        perms.ensure(Permission::TagManage)?;

        let tag_channel_id = data.tag_get_forum_id(tag_id).await?;
        if channel_id != tag_channel_id {
            return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownTag)));
        }

        let tag = data.tag_get(tag_id).await?;

        if tag.total_thread_count > 0 && !force {
            return Err(Error::Conflict);
        }

        let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
        data.tag_delete(tag_id).await?;

        if let Some(room_id) = channel.room_id {
            let al = auth.audit_log(room_id);
            al.commit_success(AuditLogEntryType::TagDelete {
                channel_id,
                tag_id,
                changes: Changes::new()
                    .remove("name", &tag.name)
                    .remove("description", &tag.description)
                    .remove("color", &tag.color)
                    .remove("restricted", &tag.restricted)
                    .build(),
            })
            .await?;
        }

        let sync_msg = MessageSync::TagDelete { channel_id, tag_id };
        self.state
            .broadcast_channel(channel_id, auth.user.id, sync_msg)
            .await?;

        Ok(())
    }

    pub async fn search(
        &self,
        channel_id: ChannelId,
        auth: &Auth,
        query: String,
        archived: Option<bool>,
        pagination: PaginationQuery<TagId>,
    ) -> Result<PaginationResponse<Tag>> {
        let perms = self
            .state
            .services()
            .perms
            .for_channel(auth.user.id, channel_id)
            .await?;
        perms.ensure(Permission::ViewChannel)?;

        let channel = self
            .state
            .services()
            .channels
            .get(channel_id, Some(auth.user.id))
            .await?;
        if !channel.ty.has_tags() {
            return Err(ApiError::from_code(ErrorCode::ChannelDoesNotSupportTags).into());
        }

        self.state
            .data()
            .tag_search(channel_id, query, archived, pagination)
            .await
    }

    pub async fn list(
        &self,
        channel_id: ChannelId,
        auth: &Auth,
        archived: Option<bool>,
        pagination: PaginationQuery<TagId>,
    ) -> Result<PaginationResponse<Tag>> {
        let perms = self
            .state
            .services()
            .perms
            .for_channel(auth.user.id, channel_id)
            .await?;
        perms.ensure(Permission::ViewChannel)?;

        let channel = self
            .state
            .services()
            .channels
            .get(channel_id, Some(auth.user.id))
            .await?;
        if !channel.ty.has_tags() {
            return Err(ApiError::from_code(ErrorCode::ChannelDoesNotSupportTags).into());
        }

        self.state
            .data()
            .tag_list(channel_id, archived, pagination)
            .await
    }
}
