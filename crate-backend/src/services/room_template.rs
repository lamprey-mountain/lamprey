use std::sync::Arc;

use common::v1::types::room_template::{
    RoomTemplate, RoomTemplateChannel, RoomTemplateCode, RoomTemplateCreate, RoomTemplatePatch,
    RoomTemplateRole, RoomTemplateSnapshot,
};
use common::v1::types::{channel::ChannelCreate, role::RoleCreate, RoomId, UserId};
use common::v1::types::{PaginationQuery, PaginationResponse};
use uuid::Uuid;

use crate::error::Result;
use crate::types::DbRoomTemplate;
use crate::{Error, ServerStateInner};

pub struct ServiceRoomTemplates {
    state: Arc<ServerStateInner>,
}

impl ServiceRoomTemplates {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    async fn hydrate(&self, db: DbRoomTemplate) -> Result<RoomTemplate> {
        let creator = self
            .state
            .services()
            .users
            .get(db.creator_id.into(), None)
            .await?;
        let snapshot: RoomTemplateSnapshot =
            serde_json::from_value(db.snapshot).map_err(|e| Error::Internal(e.to_string()))?;

        Ok(RoomTemplate {
            code: RoomTemplateCode(db.code),
            name: db.name,
            description: db.description,
            created_at: db.created_at.assume_utc().into(),
            updated_at: db.updated_at.assume_utc().into(),
            creator,
            source_room_id: db.source_room_id.map(|id| id.into()),
            snapshot,
            dirty: Some(db.dirty),
        })
    }

    /// Create a new room template from an existing room
    pub async fn create(
        &self,
        creator_id: UserId,
        create: RoomTemplateCreate,
    ) -> Result<RoomTemplate> {
        let snapshot = self.generate_room_snapshot(create.room_id).await?;
        let snapshot_json = serde_json::to_value(snapshot)?;

        let db = self
            .state
            .data()
            .room_template_create(creator_id, snapshot_json, create)
            .await?;

        self.hydrate(db).await
    }

    /// List room templates for a user
    pub async fn list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<RoomTemplateCode>,
    ) -> Result<PaginationResponse<RoomTemplate>> {
        let res = self
            .state
            .data()
            .room_template_list(user_id, pagination)
            .await?;

        let mut items = Vec::with_capacity(res.items.len());
        for item in res.items {
            items.push(self.hydrate(item).await?);
        }

        Ok(PaginationResponse {
            items,
            total: res.total,
            has_more: res.has_more,
            cursor: res.cursor,
        })
    }

    /// Get a room template by code
    pub async fn get(&self, code: RoomTemplateCode) -> Result<RoomTemplate> {
        let db = self.state.data().room_template_get(code).await?;
        self.hydrate(db).await
    }

    /// Update a room template (name, description)
    pub async fn update(
        &self,
        code: RoomTemplateCode,
        patch: RoomTemplatePatch,
    ) -> Result<RoomTemplate> {
        let db = self.state.data().room_template_update(code, patch).await?;
        self.hydrate(db).await
    }

    /// Delete a room template
    pub async fn delete(&self, code: RoomTemplateCode) -> Result<()> {
        self.state.data().room_template_delete(code).await
    }

    /// Sync a room template with its source room
    pub async fn sync(&self, code: RoomTemplateCode) -> Result<RoomTemplate> {
        let template = self.state.data().room_template_get(code.clone()).await?;
        let source_room_id = template.source_room_id.ok_or(Error::NotFound)?;

        let snapshot = self.generate_room_snapshot(source_room_id.into()).await?;
        let snapshot_json = serde_json::to_value(snapshot)?;

        let db = self
            .state
            .data()
            .room_template_update_snapshot(code, snapshot_json)
            .await?;

        self.hydrate(db).await
    }

    /// Generate a room template snapshot from an existing room using cached data
    async fn generate_room_snapshot(&self, room_id: RoomId) -> Result<RoomTemplateSnapshot> {
        use common::v1::types::channel::ChannelType;

        let cached_room = self.state.services().cache.load_room(room_id).await?;

        let mut template_channels: Vec<RoomTemplateChannel> = Vec::new();
        let mut channel_map: std::collections::HashMap<common::v1::types::ChannelId, Uuid> =
            std::collections::HashMap::new();

        for cached_channel in cached_room.channels.iter() {
            let temp_id = Uuid::now_v7();
            channel_map.insert(cached_channel.key().clone(), temp_id);
        }

        for cached_channel in cached_room.channels.iter() {
            let cc = cached_channel.value();
            let channel = &cc.inner;

            if matches!(
                channel.ty,
                ChannelType::ThreadPublic | ChannelType::ThreadPrivate
            ) {
                continue;
            }

            let temp_id = channel_map[&channel.id];

            let channel_create = ChannelCreate {
                name: channel.name.clone(),
                description: channel.description.clone(),
                ty: channel.ty,
                nsfw: channel.nsfw,
                parent_id: channel.parent_id,
                permission_overwrites: channel.permission_overwrites.clone(),
                ..Default::default()
            };

            template_channels.push(RoomTemplateChannel {
                inner: channel_create,
                id: temp_id,
            });
        }

        let mut template_roles: Vec<RoomTemplateRole> = Vec::new();

        for cached_role in cached_room.roles.iter() {
            let cr = cached_role.value();
            let role = &cr.inner;

            if role.room_id != room_id {
                continue;
            }

            let temp_id = Uuid::now_v7();

            let role_create = RoleCreate {
                name: role.name.clone(),
                description: role.description.clone(),
                allow: role.allow.clone(),
                deny: role.deny.clone(),
                is_self_applicable: role.is_self_applicable,
                is_mentionable: role.is_mentionable,
                hoist: role.hoist,
                sticky: role.sticky,
            };

            template_roles.push(RoomTemplateRole {
                inner: role_create,
                id: temp_id,
            });
        }

        let welcome_channel_id = cached_room
            .channels
            .iter()
            .find(|cc| {
                if let Some(wc) = cached_room.inner.blocking_read().welcome_channel_id {
                    cc.value().inner.id == wc
                } else {
                    false
                }
            })
            .map(|cc| cc.key().clone());

        Ok(RoomTemplateSnapshot {
            channels: template_channels,
            roles: template_roles,
            welcome_channel_id,
        })
    }
}
