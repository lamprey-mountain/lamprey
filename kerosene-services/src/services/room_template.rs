use std::collections::HashMap;

use common::v1::types::defaults::{ADMIN_ROOM, EVERYONE_TRUSTED, EVERYONE_UNTRUSTED, MODERATOR};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::room_template::{
    RoomTemplate, RoomTemplateChannel, RoomTemplateCode, RoomTemplateCreate, RoomTemplatePatch,
    RoomTemplateRole, RoomTemplateSnapshot,
};
use common::v1::types::{Channel, ChannelId, ChannelType, PermissionOverwriteType, Role, RoleId};
use common::v1::types::{PaginationQuery, PaginationResponse};
use common::v1::types::{RoomId, RoomPatch, UserId, channel::ChannelCreate, role::RoleCreate};
use uuid::Uuid;

use crate::prelude::*;
use crate::types::{DbChannelCreate, DbChannelType, DbRoleCreate, DbRoomTemplate};

pub mod builtin;

pub struct ServiceRoomTemplates {
    globals: Globals,
}

impl ServiceRoomTemplates {
    pub fn new(globals: Globals) -> Self {
        Self { globals }
    }

    async fn hydrate(&self, db: DbRoomTemplate) -> Result<RoomTemplate> {
        let srv = self.globals.services();
        let creator = srv.users.get(db.creator_id.into(), None).await?;
        let snapshot: RoomTemplateSnapshot =
            serde_json::from_value(db.snapshot).map_err(|e| Error::Internal(e.to_string()))?;
        // FIXME: convert strip dirty and source_room_id if needed
        // let creator = srv.perms.for_room3(None, room_id);
        // .get(db.creator_id.into(), None).await?;

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

        let mut data = self.globals.begin().await?;
        let db = data
            .room_template_create(creator_id, snapshot_json, create)
            .await?;
        data.commit().await?;

        self.hydrate(db).await
    }

    /// List room templates for a user
    pub async fn list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<RoomTemplateCode>,
    ) -> Result<PaginationResponse<RoomTemplate>> {
        let mut data = self.globals.begin_read().await?;
        let res = data.room_template_list(user_id, pagination).await?;

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
        let mut data = self.globals.begin_read().await?;
        let db = data.room_template_get(code).await?;
        self.hydrate(db).await
    }

    /// Update a room template (name, description)
    pub async fn update(
        &self,
        code: RoomTemplateCode,
        patch: RoomTemplatePatch,
    ) -> Result<RoomTemplate> {
        let mut data = self.globals.begin().await?;
        let db = data.room_template_update(code, patch).await?;
        data.commit().await?;

        self.hydrate(db).await
    }

    /// Delete a room template
    pub async fn delete(&self, code: RoomTemplateCode) -> Result<()> {
        let mut data = self.globals.begin().await?;
        data.room_template_delete(code).await?;
        data.commit().await
    }

    /// Sync a room template with its source room
    pub async fn sync(&self, code: RoomTemplateCode) -> Result<RoomTemplate> {
        let mut data = self.globals.begin().await?;
        let template = data.room_template_get(code.clone()).await?;
        let source_room_id =
            template
                .source_room_id
                .ok_or(Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownRoomTemplate,
                )))?;

        let snapshot = self.generate_room_snapshot(source_room_id.into()).await?;
        let snapshot_json = serde_json::to_value(snapshot)?;

        let db = data
            .room_template_update_snapshot(code, snapshot_json)
            .await?;
        data.commit().await?;

        self.hydrate(db).await
    }

    pub async fn apply_to_room(
        &self,
        room_id: RoomId,
        creator_id: UserId,
        snapshot: RoomTemplateSnapshot,
    ) -> Result<(Vec<Role>, Vec<Channel>)> {
        let mut data = self.globals.begin().await?;
        let mut role_map = HashMap::new();
        let mut channel_map = HashMap::new();
        let mut created_roles = Vec::new();
        let mut created_channels = Vec::new();

        // Create roles
        for template_role in &snapshot.roles {
            let role_id = if template_role.position == 0 {
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
                    template_role.position as u64,
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
                    PermissionOverwriteType::Role => **role_map
                        .get(&overwrite.id)
                        .ok_or_else(|| Error::Internal("failed to create role".to_string()))?,
                    PermissionOverwriteType::User => *overwrite.id,
                };
                data.permission_overwrite_upsert(
                    channel_id,
                    target_id,
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

        data.commit().await?;
        Ok((created_roles, created_channels))
    }

    /// Generate a room template snapshot from an existing room using cached data
    async fn generate_room_snapshot(&self, room_id: RoomId) -> Result<RoomTemplateSnapshot> {
        let srv = self.globals.services();
        let handle = srv.rooms.load(room_id);
        let r = handle.ready(false).await?;

        let mut channels: HashMap<ChannelId, RoomTemplateChannel> = HashMap::new();
        let mut roles: HashMap<RoleId, RoomTemplateRole> = HashMap::new();

        // collect channels
        for (id, cc) in &r.channels {
            let chan = &cc.inner;

            if chan.is_thread() {
                continue;
            }

            let create = ChannelCreate {
                name: chan.name.clone(),
                description: chan.description.clone(),
                ty: chan.ty,
                nsfw: chan.nsfw,
                parent_id: chan.parent_id,
                permission_overwrites: chan.permission_overwrites.clone(),
                url: chan.url.clone(),
                bitrate: chan.bitrate,
                user_limit: chan.user_limit,
                default_auto_archive_duration: chan.default_auto_archive_duration,
                slowmode_thread: chan.slowmode_thread,
                slowmode_message: chan.slowmode_message,
                default_slowmode_message: chan.default_slowmode_message,
                ..Default::default()
            };

            channels.insert(
                *id,
                RoomTemplateChannel {
                    inner: create,
                    id: Uuid::now_v7(),
                    // TODO: warn!() if None
                    position: chan.position.unwrap_or_default(),
                },
            );
        }

        // collect roles
        for (id, cr) in &r.roles {
            let create = RoleCreate {
                name: cr.name.to_string(),
                description: cr.description.as_ref().map(|s| s.to_string()),
                allow: cr.allow.to_vec(),
                deny: cr.deny.to_vec(),
                is_self_applicable: cr.is_self_applicable(),
                is_mentionable: cr.is_mentionable(),
                hoist: cr.hoist(),
                sticky: cr.sticky(),
            };

            roles.insert(
                *id,
                RoomTemplateRole {
                    inner: create,
                    id: Uuid::now_v7(),
                    position: cr.position,
                },
            );
        }

        // rewrite to use temporary ids
        let channel_map: HashMap<ChannelId, Uuid> =
            channels.iter().map(|(id, c)| (*id, c.id)).collect();
        let role_map: HashMap<RoleId, Uuid> = roles.iter().map(|(id, r)| (*id, r.id)).collect();

        for chan in channels.values_mut() {
            if let Some(parent_id) = &mut chan.inner.parent_id {
                *parent_id = channel_map[&(*parent_id).into()].into();
            }

            for ow in &mut chan.inner.permission_overwrites {
                if ow.ty == PermissionOverwriteType::Role {
                    ow.id = role_map[&ow.id.into()].into();
                }
            }
        }

        let welcome_channel_id = r.room.welcome_channel_id.map(|id| channel_map[&id].into());
        let afk_channel_id = r.room.afk_channel_id.map(|id| channel_map[&id].into());

        Ok(RoomTemplateSnapshot {
            channels: channels.into_values().collect(),
            roles: roles.into_values().collect(),
            welcome_channel_id,
            afk_channel_id,
            afk_channel_timeout: r.room.afk_channel_timeout,
        })
    }
}
