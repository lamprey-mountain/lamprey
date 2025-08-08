use std::sync::Arc;

use common::v1::types::defaults::{EVERYONE_TRUSTED, MODERATOR};
use common::v1::types::util::Diff;
use common::v1::types::{
    AuditLogChange, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Permission, Room,
    RoomCreate, RoomId, RoomMembership, RoomPatch, UserId,
};
use moka::future::Cache;

use crate::error::{Error, Result};
use crate::types::DbRoleCreate;
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

    pub async fn update(&self, room_id: RoomId, user_id: UserId, patch: RoomPatch) -> Result<Room> {
        let data = self.state.data();
        let start = data.room_get(room_id).await?;
        if !patch.changes(&start) {
            return Err(Error::NotModified);
        }

        let changes = vec![
            AuditLogChange {
                key: "name".to_string(),
                old: serde_json::to_value(&start.name).unwrap(),
                new: serde_json::to_value(&patch.name).unwrap(),
            },
            AuditLogChange {
                key: "description".to_string(),
                old: serde_json::to_value(&start.description).unwrap(),
                new: serde_json::to_value(&patch.description).unwrap(),
            },
            AuditLogChange {
                key: "icon".to_string(),
                old: serde_json::to_value(&start.icon).unwrap(),
                new: serde_json::to_value(&patch.icon).unwrap(),
            },
            AuditLogChange {
                key: "public".to_string(),
                old: serde_json::to_value(&start.public).unwrap(),
                new: serde_json::to_value(&patch.public).unwrap(),
            },
        ];

        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id,
            session_id: None, // TODO: get session id
            reason: None,     // TODO: get reason
            ty: AuditLogEntryType::RoomUpdate { changes },
        })
        .await?;

        data.room_update(room_id, patch).await?;
        self.cache_room.invalidate(&room_id).await;
        self.get(room_id, Some(user_id)).await
    }

    pub async fn create(&self, create: RoomCreate, creator: UserId) -> Result<Room> {
        let data = self.state.data();
        let room = data.room_create(create).await?;
        let room_id = room.id;

        let changes = vec![
            AuditLogChange {
                key: "name".to_string(),
                old: serde_json::Value::Null,
                new: serde_json::to_value(&room.name).unwrap(),
            },
            AuditLogChange {
                key: "description".to_string(),
                old: serde_json::Value::Null,
                new: serde_json::to_value(&room.description).unwrap(),
            },
            AuditLogChange {
                key: "icon".to_string(),
                old: serde_json::Value::Null,
                new: serde_json::to_value(&room.icon).unwrap(),
            },
            AuditLogChange {
                key: "public".to_string(),
                old: serde_json::Value::Null,
                new: serde_json::to_value(&room.public).unwrap(),
            },
        ];

        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: creator,
            session_id: None, // TODO: get session id
            reason: None,     // TODO: get reason
            ty: AuditLogEntryType::RoomCreate { changes },
        })
        .await?;

        let role_admin = DbRoleCreate {
            room_id,
            name: "admin".to_owned(),
            description: None,
            permissions: vec![Permission::Admin],
            is_self_applicable: false,
            is_mentionable: false,
            is_default: false,
        };
        let role_moderator = DbRoleCreate {
            room_id,
            name: "moderator".to_owned(),
            description: None,
            permissions: MODERATOR.to_vec(),
            is_self_applicable: false,
            is_mentionable: false,
            is_default: false,
        };
        let role_everyone = DbRoleCreate {
            room_id,
            name: "everyone".to_owned(),
            description: None,
            permissions: EVERYONE_TRUSTED.to_vec(),
            is_self_applicable: false,
            is_mentionable: false,
            is_default: true,
        };
        let admin = data.role_create(role_admin).await?;
        data.role_create(role_moderator).await?;
        data.role_create(role_everyone).await?;
        data.room_member_put(
            room_id,
            creator,
            RoomMembership::Join {
                override_name: None,
                override_description: None,
                roles: vec![],
            },
        )
        .await?;
        data.role_member_put(creator, admin.id).await?;
        data.role_apply_default(room.id, creator).await?;
        Ok(room)
    }
}
