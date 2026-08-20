use lamprey_macros::record;

use crate::v1::types::{UserId, util::Diff};

#[cfg(feature = "serde")]
use crate::v1::types::util::{deserialize_sorted, deserialize_sorted_option, some_option};

use super::{Permission, RoleId, RoleVerId, RoomId};

#[record]
#[derive(PartialEq, Eq)]
pub struct Role {
    pub id: RoleId,
    pub version_id: RoleVerId,
    pub room_id: RoomId,

    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    #[schema(min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    pub description: Option<String>,

    /// the permissions to grant for this role
    #[serde(deserialize_with = "deserialize_sorted", alias = "permissions")]
    pub allow: Vec<Permission>,

    /// the permissions to deny for this role
    #[serde(default, deserialize_with = "deserialize_sorted")]
    pub deny: Vec<Permission>,

    // TODO: remove is_
    pub is_self_applicable: bool,
    pub is_mentionable: bool,

    /// tiebroken by id
    pub position: u64,

    /// whether members with this role should be displayed separately
    pub hoist: bool,

    /// whether this role should be retained after a user leaves and rejoins the room
    pub sticky: bool,

    pub member_count: u64,
}

#[record]
#[derive(Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct RoleDeleteQuery {
    pub fallback_role_id: Option<RoleId>,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct RoleCreate {
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    #[schema(required = false, min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub allow: Vec<Permission>,

    #[serde(default)]
    pub deny: Vec<Permission>,

    #[serde(default)]
    pub is_self_applicable: bool,

    /// if this role can be mentioned by members
    #[serde(default)]
    pub is_mentionable: bool,

    #[serde(default)]
    pub hoist: bool,

    #[serde(default)]
    pub sticky: bool,
    // the main reason this doesn't exist yet is because i've seen in
    // discord how the ui can become extremely unreadable, cluttered, and
    // in general color vomit. plus there's the whole "illegable contrast
    // in light/dark mode" thing.
    //
    // i also don't really like the psychological effects of colored names,
    // since i've seen people act differently when someone with a differently
    // colored name shows up (eg. moderators)
    //
    // still, it can be very useful. i'm not sure what's the best way to
    // implement this though; definitely not copying discord here.
    //
    // pub color: Color,
}

#[record]
#[derive(PartialEq, Eq, Diff)]
pub struct RolePatch {
    #[schema(required = false, min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub name: Option<String>,

    #[schema(min_length = 1, max_length = 8192)]
    #[validate(length(min = 1, max = 8192))]
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,

    #[serde(default, deserialize_with = "deserialize_sorted_option")]
    pub allow: Option<Vec<Permission>>,

    #[serde(default, deserialize_with = "deserialize_sorted_option")]
    pub deny: Option<Vec<Permission>>,

    pub is_self_applicable: Option<bool>,
    pub is_mentionable: Option<bool>,
    pub hoist: Option<bool>,
    pub sticky: Option<bool>,
}

/// apply and remove a role to many members at once
#[record]
#[derive(PartialEq, Eq)]
pub struct RoleMemberBulkPatch {
    /// add this role to these users
    #[serde(default)]
    #[validate(length(min = 1, max = 256))]
    pub apply: Vec<UserId>,

    /// remove this role from these users
    #[serde(default)]
    #[validate(length(min = 1, max = 256))]
    pub remove: Vec<UserId>,
}

/// reorder some roles
#[record]
#[derive(PartialEq, Eq)]
pub struct RoleReorder {
    /// the roles to reorder
    #[serde(default)]
    #[validate(length(min = 1, max = 1024))]
    pub roles: Vec<RoleReorderItem>,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct RoleReorderItem {
    pub role_id: RoleId,
    pub position: u64,
}

impl Role {
    /// returns if this is the default/everyone role that everyone in a room implicitly has
    pub fn is_default(&self) -> bool {
        *self.id == *self.room_id
    }
}
