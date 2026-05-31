//! various admin-only apis

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{MessageCreate, UserId};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AdminWhisper {
    pub user_id: UserId,
    pub message: MessageCreate,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AdminRegisterUser {
    pub user_id: UserId,
}
