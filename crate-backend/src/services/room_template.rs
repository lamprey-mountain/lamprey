use std::collections::HashMap;
use std::sync::Arc;

use common::v1::types::defaults::{ADMIN_ROOM, EVERYONE_TRUSTED, EVERYONE_UNTRUSTED, MODERATOR};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::room_template::{
    RoomTemplate, RoomTemplateChannel, RoomTemplateCode, RoomTemplateCreate, RoomTemplatePatch,
    RoomTemplateRole, RoomTemplateSnapshot,
};
use common::v1::types::{channel::ChannelCreate, role::RoleCreate, RoomId, RoomPatch, UserId};
use common::v1::types::{Channel, ChannelId, ChannelType, PermissionOverwriteType, Role, RoleId};
use common::v1::types::{PaginationQuery, PaginationResponse};
use uuid::Uuid;

use crate::error::Result;
use crate::types::{DbChannelCreate, DbChannelType, DbRoleCreate, DbRoomTemplate};
use crate::{Error, ServerStateInner};

pub mod builtin {
    use super::*;

    pub fn public_room() -> RoomTemplateSnapshot {
        room_snapshot(true)
    }

    pub fn private_room() -> RoomTemplateSnapshot {
        room_snapshot(false)
    }

    fn room_snapshot(public: bool) -> RoomTemplateSnapshot {
        let everyone_id = Uuid::now_v7();
        let general_id = Uuid::now_v7();

        RoomTemplateSnapshot {
            roles: vec![
                RoomTemplateRole {
                    id: Uuid::now_v7(),
                    inner: RoleCreate {
                        name: "admin".to_string(),
                        description: None,
                        allow: ADMIN_ROOM.to_vec(),
                        deny: vec![],
                        is_self_applicable: false,
                        is_mentionable: false,
                        hoist: false,
                        sticky: false,
                    },
                    default: false,
                    position: 2,
                },
                RoomTemplateRole {
                    id: Uuid::now_v7(),
                    inner: RoleCreate {
                        name: "moderator".to_string(),
                        description: None,
                        allow: MODERATOR.to_vec(),
                        deny: vec![],
                        is_self_applicable: false,
                        is_mentionable: false,
                        hoist: false,
                        sticky: false,
                    },
                    default: false,
                    position: 1,
                },
                RoomTemplateRole {
                    id: everyone_id,
                    inner: RoleCreate {
                        name: "everyone".to_string(),
                        description: Some("Default role".to_string()),
                        allow: if public {
                            EVERYONE_UNTRUSTED.to_vec()
                        } else {
                            EVERYONE_TRUSTED.to_vec()
                        },
                        deny: vec![],
                        is_self_applicable: false,
                        is_mentionable: false,
                        hoist: false,
                        sticky: false,
                    },
                    default: true,
                    position: 0,
                },
            ],
            channels: vec![RoomTemplateChannel {
                id: general_id,
                inner: ChannelCreate {
                    name: "general".to_string(),
                    ty: ChannelType::Text,
                    ..Default::default()
                },
            }],
            welcome_channel_id: Some(general_id.into()),
        }
    }
}

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
        let source_room_id =
            template
                .source_room_id
                .ok_or(Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownRoomTemplate,
                )))?;

        let snapshot = self.generate_room_snapshot(source_room_id.into()).await?;
        let snapshot_json = serde_json::to_value(snapshot)?;

        let db = self
            .state
            .data()
            .room_template_update_snapshot(code, snapshot_json)
            .await?;

        self.hydrate(db).await
    }

    pub async fn apply_to_room(
        &self,
        room_id: RoomId,
        creator_id: UserId,
        snapshot: RoomTemplateSnapshot,
    ) -> Result<(Vec<Role>, Vec<Channel>)> {
        let data = self.state.data();
        let mut role_map = HashMap::new();
        let mut channel_map = HashMap::new();
        let mut created_roles = Vec::new();
        let mut created_channels = Vec::new();

        // Create roles
        for template_role in &snapshot.roles {
            let role_id = if template_role.default {
                RoleId::from(room_id.into_inner())
            } else {
                RoleId::new()
            };

            let role = data
                .role_create(
                    DbRoleCreate {
                        id: role_id,
                        room_id,
                        name: template_role.inner.name.clone(),
                        description: template_role.inner.description.clone(),
                        allow: template_role.inner.allow.clone(),
                        deny: template_role.inner.deny.clone(),
                        is_self_applicable: template_role.inner.is_self_applicable,
                        is_mentionable: template_role.inner.is_mentionable,
                        hoist: template_role.inner.hoist,
                        sticky: template_role.inner.sticky,
                    },
                    template_role.position,
                )
                .await?;

            role_map.insert(template_role.id, role.id);
            created_roles.push(role);
        }

        // Create channels
        for template_channel in &snapshot.channels {
            let channel_id = data
                .channel_create(DbChannelCreate {
                    room_id: Some(room_id.into_inner()),
                    creator_id,
                    name: template_channel.inner.name.clone(),
                    description: template_channel.inner.description.clone(),
                    ty: DbChannelType::from(template_channel.inner.ty),
                    nsfw: template_channel.inner.nsfw,
                    bitrate: template_channel.inner.bitrate.map(|b| b as i32),
                    user_limit: template_channel.inner.user_limit.map(|u| u as i32),
                    parent_id: template_channel.inner.parent_id.and_then(|p| {
                        channel_map
                            .get(&p.into_inner())
                            .copied()
                            .map(|id: ChannelId| id.into_inner())
                    }),
                    owner_id: None,
                    icon: template_channel.inner.icon.map(|i| *i),
                    invitable: template_channel.inner.invitable,
                    auto_archive_duration: template_channel
                        .inner
                        .auto_archive_duration
                        .map(|d| d as i64),
                    default_auto_archive_duration: template_channel
                        .inner
                        .default_auto_archive_duration
                        .map(|d| d as i64),
                    slowmode_thread: template_channel.inner.slowmode_thread.map(|d| d as i64),
                    slowmode_message: template_channel.inner.slowmode_message.map(|d| d as i64),
                    default_slowmode_message: template_channel
                        .inner
                        .default_slowmode_message
                        .map(|d| d as i64),
                    tags: template_channel.inner.tags.clone(),
                    url: template_channel.inner.url.clone(),
                    locked: false,
                })
                .await?;

            channel_map.insert(template_channel.id, channel_id);

            // Apply overwrites
            for overwrite in &template_channel.inner.permission_overwrites {
                let target_id = match overwrite.ty {
                    PermissionOverwriteType::Role => *role_map
                        .get(&overwrite.id)
                        .ok_or_else(|| Error::Internal("failed to create role".to_string()))?,
                    PermissionOverwriteType::User => overwrite.id.into(),
                };
                data.permission_overwrite_upsert(
                    channel_id,
                    *target_id,
                    overwrite.ty,
                    overwrite.allow.clone(),
                    overwrite.deny.clone(),
                )
                .await?;
            }

            let channel = data.channel_get(channel_id).await?;
            created_channels.push(channel);
        }

        // Set welcome channel
        if let Some(welcome_placeholder) = snapshot.welcome_channel_id {
            if let Some(welcome_id) = channel_map.get(&welcome_placeholder.into_inner()) {
                data.room_update(
                    room_id,
                    RoomPatch {
                        welcome_channel_id: Some(Some(*welcome_id)),
                        ..Default::default()
                    },
                )
                .await?;
            }
        } else if let Some(first_channel) = snapshot.channels.first() {
            if let Some(welcome_id) = channel_map.get(&first_channel.id) {
                data.room_update(
                    room_id,
                    RoomPatch {
                        welcome_channel_id: Some(Some(*welcome_id)),
                        ..Default::default()
                    },
                )
                .await?;
            }
        }

        Ok((created_roles, created_channels))
    }

    /// Generate a room template snapshot from an existing room using cached data
    async fn generate_room_snapshot(&self, room_id: RoomId) -> Result<RoomTemplateSnapshot> {
        use common::v1::types::channel::ChannelType;

        let snapshot = self.state.services().cache.load_room(room_id, false).await?;
        let data = snapshot.get_data().unwrap();

        let mut template_channels: Vec<RoomTemplateChannel> = Vec::new();
        let mut channel_map: HashMap<ChannelId, Uuid> = HashMap::new();

        for channel_id in data.channels.keys() {
            let temp_id = Uuid::now_v7();
            channel_map.insert(*channel_id, temp_id);
        }

        for cc in data.channels.values() {
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

        for cr in data.roles.values() {
            let role = &cr.inner;

            if role.room_id != room_id {
                continue;
            }

            let temp_id = Uuid::now_v7();
            let is_default = role.id.into_inner() == room_id.into_inner();

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
                default: is_default,
                position: role.position,
            });
        }

        let welcome_channel_id = data
            .channels
            .values()
            .find(|cc| {
                if let Some(wc) = data.room.welcome_channel_id {
                    cc.inner.id == wc
                } else {
                    false
                }
            })
            .map(|cc| cc.inner.id);

        Ok(RoomTemplateSnapshot {
            channels: template_channels,
            roles: template_roles,
            welcome_channel_id,
        })
    }
}
