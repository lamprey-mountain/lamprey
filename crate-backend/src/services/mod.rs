use types::{Permission, Room, RoomCreate, RoomMemberPut, RoomMembership, UserId};

use crate::{data::Data, types::RoleCreate};
use crate::error::Result;

pub struct Services {
    data: Box<dyn Data>,
}

#[allow(async_fn_in_trait)]
impl Services {
    pub fn new(data: Box<dyn Data>) -> Self {
        Self { data }
    }

    pub async fn create_room(&self, create: RoomCreate, creator: UserId) -> Result<Room> {
        let room = self.data.room_create(create).await?;
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
        let admin = self.data.role_create(role_admin).await?;
        self.data.role_create(role_moderator).await?;
        self.data.role_create(role_everyone).await?;
        self.data
            .room_member_put(RoomMemberPut {
                user_id: creator,
                room_id,
                membership: RoomMembership::Join,
                override_name: None,
                override_description: None,
                roles: vec![],
            })
            .await?;
        self.data.role_member_put(creator, admin.id).await?;
        self.data.role_apply_default(room.id, creator).await?;
        Ok(room)
    }
}
