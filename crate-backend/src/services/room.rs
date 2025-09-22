use std::sync::Arc;

use common::v1::types::defaults::{EVERYONE_TRUSTED, MODERATOR};
use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Permission, RoleId, Room, RoomCreate,
    RoomId, RoomMemberOrigin, RoomMemberPut, RoomPatch, UserId,
};
use moka::future::Cache;

use crate::error::{Error, Result};
use crate::types::{DbRoleCreate, DbRoomCreate, MediaLinkType};
use crate::ServerStateInner;

pub struct ServiceRooms {
    state: Arc<ServerStateInner>,
    cache_room: Cache<RoomId, Room>,
}

impl ServiceRooms {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache_room: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
        }
    }

    pub async fn get(&self, room_id: RoomId, _user_id: Option<UserId>) -> Result<Room> {
        self.cache_room
            .try_get_with(room_id, self.state.data().room_get(room_id))
            .await
            .map_err(|err| err.fake_clone())
    }

    pub async fn invalidate(&self, room_id: RoomId) {
        self.cache_room.invalidate(&room_id).await;
    }

    pub async fn update(
        &self,
        room_id: RoomId,
        user_id: UserId,
        patch: RoomPatch,
        reason: Option<String>,
    ) -> Result<Room> {
        let data = self.state.data();
        let start = data.room_get(room_id).await?;
        if !patch.changes(&start) {
            return Err(Error::NotModified);
        }

        if let Some(icon) = &patch.icon {
            if start.icon.is_some() {
                data.media_link_delete_all(*room_id).await?;
            }
            if let Some(media_id) = icon {
                data.media_link_insert(*media_id, *room_id, MediaLinkType::AvatarRoom)
                    .await?;
            }
        }

        data.room_update(room_id, patch).await?;

        self.cache_room.invalidate(&room_id).await;
        let end = self.get(room_id, Some(user_id)).await?;

        let changes = Changes::new()
            .change("name", &start.name, &end.name)
            .change("description", &start.description, &end.description)
            .change("icon", &start.icon, &end.icon)
            .change("public", &start.public, &end.public)
            .build();

        self.state
            .audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id,
                session_id: None, // TODO: get session id
                reason,
                ty: AuditLogEntryType::RoomUpdate { changes },
            })
            .await?;

        Ok(end)
    }

    pub async fn create(
        &self,
        create: RoomCreate,
        creator: UserId,
        extra: DbRoomCreate,
    ) -> Result<Room> {
        let data = self.state.data();
        let mut room = data.room_create(create, extra).await?;
        let room_id = room.id;

        let changes = Changes::new()
            .add("name", &room.name)
            .add("description", &room.description)
            .add("icon", &room.icon)
            .add("public", &room.public)
            .build();

        self.state
            .audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: creator,
                session_id: None, // TODO: get session id
                reason: None,     // TODO: get reason
                ty: AuditLogEntryType::RoomCreate { changes },
            })
            .await?;

        let role_admin = DbRoleCreate {
            id: RoleId::new(),
            room_id,
            name: "admin".to_owned(),
            description: None,
            permissions: vec![Permission::Admin],
            is_self_applicable: false,
            is_mentionable: false,
        };
        let role_moderator = DbRoleCreate {
            id: RoleId::new(),
            room_id,
            name: "moderator".to_owned(),
            description: None,
            permissions: MODERATOR.to_vec(),
            is_self_applicable: false,
            is_mentionable: false,
        };
        let role_everyone = DbRoleCreate {
            id: RoleId::from(room.id.into_inner()),
            room_id,
            name: "everyone".to_owned(),
            description: Some("Default role".to_string()),
            permissions: EVERYONE_TRUSTED.to_vec(),
            is_self_applicable: false,
            is_mentionable: false,
        };
        data.role_create(role_admin, 1).await?;
        data.role_create(role_moderator, 1).await?;
        data.role_create(role_everyone, 0).await?;
        data.room_member_put(
            room_id,
            creator,
            Some(RoomMemberOrigin::Creator),
            RoomMemberPut::default(),
        )
        .await?;
        data.room_set_owner(room_id, creator).await?;
        room.owner_id = Some(creator);
        Ok(room)
    }
}
