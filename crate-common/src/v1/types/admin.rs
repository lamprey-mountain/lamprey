//! various admin-only apis

use lamprey_macros::record;

use crate::v1::types::{MessageCreate, UserId};

#[record]
pub struct AdminWhisper {
    pub user_id: UserId,
    pub message: MessageCreate,
}

#[record]
pub struct AdminBroadcast {
    pub message: MessageCreate,
    // TODO: add these
    // /// only broadcast to users in these rooms
    // room_id: Vec<RoomId>,

    // /// only broadcast to these users
    // user_id: Vec<UserId>,

    // /// only broadcast to these users with these server roles
    // server_roles: Vec<RoleId>,
}

#[record]
pub struct AdminRegisterUser {
    pub user_id: UserId,
}
