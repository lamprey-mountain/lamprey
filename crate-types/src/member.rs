use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{Role, RoleId, RoomId, User, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomMember {
    pub user: User,
    pub room_id: RoomId,
    pub membership: RoomMembership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    // override_avatar: z.string().url().or(z.literal("")),
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomMemberPut {
    pub user_id: UserId,
    pub room_id: RoomId,
    pub membership: RoomMembership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    // override_avatar: z.string().url().or(z.literal("")),
    pub roles: Vec<RoleId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RoomMembership {
    // #[default]
    Join,
    Ban,
}

// === future stuff ===

struct RoomMemberPut2 {
    pub user_id: UserId,
    pub room_id: RoomId,
    pub version_id: RoomMemberVersionId,
    pub version_from: UserId, // who updated this member
    pub state: RoomMembership,
}

enum RoomMemberState {
    /// joined
    Join {
        override_name: Option<String>,
        override_description: Option<String>,
        override_avatar: Option<String>,
        roles: Vec<RoleId>,
    },
    
    /// kicked or left, can still view messages up until then, can rejoin
    Left {
        reason: Option<String>,
    },
    
    /// banned, can still view messages up until they were banned
    Ban {
        reason: Option<String>,
    },
}
