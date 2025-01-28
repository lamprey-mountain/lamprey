use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{Role, RoleId, RoomId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomMember {
    pub user_id: UserId,
    pub room_id: RoomId,
    pub membership: RoomMembership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    // override_avatar: z.string().url().or(z.literal("")),
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
pub struct RoomMemberPatch {
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    // override_avatar: z.string().url().or(z.literal("")),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RoomMembership {
    // #[default]
    Join,
    Ban,
}

// === future stuff ===

// pub struct RoomMember2 {
//     pub user: User,
//     pub room_id: RoomId,
//     pub membership: RoomMembership,
//     pub override_name: Option<String>,
//     pub override_description: Option<String>,
//     // pub override_avatar: Option<String>,
//     pub roles: Vec<Role>,
//     pub joined_at: time::OffsetDateTime,
// }

// generic profile data thing
// struct Profile {
//     name: String,
//     /// room = topic, user = status
//     info_short: Option<String>,
//     /// room = description, user = bio
//     info_long: Option<String>,
//     avatar: Option<Url>,
//     banner: Option<Url>,
//     /// list of preferred locales, in order of most to least preferred
//     languages: Vec<Locale>,
// }

// struct RoomMemberPut2 {
//     pub user_id: UserId,
//     pub room_id: RoomId,
//     pub version_id: RoomMemberVersionId,
//     pub version_from: UserId, // who updated this member
//     pub state: RoomMembership,
// }

// enum RoomMemberState {
//     /// joined
//     Join {
//         override_name: Option<String>,
//         override_description: Option<String>,
//         override_avatar: Option<String>,
//         roles: Vec<RoleId>,
//     },

//     /// kicked or left, can still view messages up until then, can rejoin
//     Left {
//         reason: Option<String>,
//     },

//     /// banned, can still view messages up until they were banned
//     Ban {
//         reason: Option<String>,
//     },
// }
