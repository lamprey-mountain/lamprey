use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{Role, RoleId, RoomId, User, UserId};

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct RoomMember {
    pub user: User,
    pub room_id: RoomId,
    pub membership: Membership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    // override_avatar: z.string().url().or(z.literal("")),
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMemberPut {
    pub user_id: UserId,
    pub room_id: RoomId,
    pub membership: Membership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    // override_avatar: z.string().url().or(z.literal("")),
    pub roles: Vec<RoleId>,
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "membership")]
pub enum Membership {
    // #[default]
    Join,
    Ban,
}
