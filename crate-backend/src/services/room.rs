use std::sync::Arc;

use types::{Permission, Room, RoomCreate, RoomMembership, UserId};

use crate::error::Result;
use crate::types::RoleCreate;
use crate::ServerState;

pub struct ServiceRooms {
    state: Arc<ServerState>,
}

impl ServiceRooms {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
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
        data
            .room_member_put(
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
