use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{Role, RoomId, User};

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Member {
    pub user: User,
    pub room_id: RoomId,
    pub membership: Membership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    // override_avatar: z.string().url().or(z.literal("")),
    pub roles: Vec<Role>,
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "membership")]
pub enum Membership {
    // #[default]
    Join,
    Ban,
}
