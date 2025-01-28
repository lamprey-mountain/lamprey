use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{Permission, RoleId, RoleVerId, RoomId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Role {
    pub id: RoleId,
    pub version_id: RoleVerId,
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    // TODO: always keep sorted
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
    pub description: Option<Option<String>>,
    pub permissions: Option<Vec<Permission>>,
    pub is_self_applicable: Option<bool>,
    pub is_mentionable: Option<bool>,
    pub is_default: Option<bool>,
}

impl RolePatch {
    pub fn wont_change(&self, target: &Role) -> bool {
        self.name.as_ref().is_none_or(|c| c == &target.name)
            && self
                .description
                .as_ref()
                .is_none_or(|c| c == &target.description)
            && self
                .permissions
                .as_ref()
                .is_none_or(|c| c == &target.permissions)
            && self
                .is_self_applicable
                .as_ref()
                .is_none_or(|c| c == &target.is_self_applicable)
            && self
                .is_mentionable
                .as_ref()
                .is_none_or(|c| c == &target.is_mentionable)
            && self
                .is_default
                .as_ref()
                .is_none_or(|c| c == &target.is_default)
    }
}
