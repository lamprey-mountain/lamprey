use std::sync::Arc;

use moka::future::Cache;
use types::util::Diff;
use types::{Permission, Room, RoomCreate, RoomId, RoomMembership, RoomPatch, UserId};

use crate::error::{Error, Result};
use crate::types::RoleCreate;
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

        data.room_update(room_id, patch).await?;
        self.cache_room
            .remove(&room_id)
            .await
            .expect("failed to invalidate");
        self.get(room_id, Some(user_id)).await
    }

    pub async fn create(&self, create: RoomCreate, creator: UserId) -> Result<Room> {
        let data = self.state.data();
        let room = data.room_create(create).await?;
        let room_id = room.id;
        let role_admin = RoleCreate {
            room_id,
            name: "admin".to_owned(),
            description: None,
            permissions: vec![Permission::Admin],
            is_self_applicable: false,
            is_mentionable: false,
            is_default: false,
        };
        let role_moderator = RoleCreate {
            room_id,
            name: "moderator".to_owned(),
            description: None,
            permissions: vec![
                Permission::ThreadManage,
                Permission::ThreadDelete,
                Permission::MessagePin,
                Permission::MessageDelete,
                Permission::MemberKick,
                Permission::MemberBan,
                Permission::MemberManage,
                Permission::InviteManage,
            ],
            is_self_applicable: false,
            is_mentionable: false,
            is_default: false,
        };
        let role_everyone = RoleCreate {
            room_id,
            name: "everyone".to_owned(),
            description: None,
            permissions: vec![
                Permission::MessageCreate,
                Permission::MessageFilesEmbeds,
                Permission::ThreadCreate,
                Permission::InviteCreate,
            ],
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
