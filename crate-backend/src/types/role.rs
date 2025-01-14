use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{Permission, RoleId, RoomId};

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type)]
pub struct Role {
    pub id: RoleId,
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type)]
pub struct RoleCreate {
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::FromRow, sqlx::Type)]
pub struct RolePatch {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub permissions: Option<Vec<Permission>>,
    pub is_self_applicable: Option<bool>,
    pub is_mentionable: Option<bool>,
    pub is_default: Option<bool>,
}
