use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::util::{
    deserialize_sorted_permissions, deserialize_sorted_permissions_option, some_option, Diff,
};

use super::{Permission, RoleId, RoleVerId, RoomId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Role {
    pub id: RoleId,
    pub version_id: RoleVerId,
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    #[serde(deserialize_with = "deserialize_sorted_permissions")]
    pub permissions: Vec<Permission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoleCreateRequest {
    pub name: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub permissions: Vec<Permission>,

    #[serde(default)]
    pub is_self_applicable: bool,

    #[serde(default)]
    pub is_mentionable: bool,

    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RolePatch {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_sorted_permissions_option")]
    pub permissions: Option<Vec<Permission>>,
    pub is_self_applicable: Option<bool>,
    pub is_mentionable: Option<bool>,
    pub is_default: Option<bool>,
}

impl Diff<Role> for RolePatch {
    fn changes(&self, other: &Role) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.is_self_applicable.changes(&other.is_self_applicable)
            || self.is_mentionable.changes(&other.is_mentionable)
            || self.is_default.changes(&other.is_default)
            || self.permissions.changes(&other.permissions)
    }
}
