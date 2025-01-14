use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::ids::RoomId;

/// A room
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Room {
    /// A unique identifier for this room
    #[cfg_attr(feature = "utoipa", schema(read_only))]
    pub id: RoomId,

    /// A monotonically increasing id that is updated every time this room is modified.
    #[cfg_attr(feature = "utoipa", schema(read_only))]
    pub version_id: Uuid,

    #[cfg_attr(feature = "utoipa", schema(read_only))]
    pub name: String,

    #[cfg_attr(feature = "utoipa", schema(read_only, required = false))]
    pub description: Option<String>,
    // default_roles: Vec<RoleId>,
}

/// Data required to create a room
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomCreate {
    #[cfg_attr(feature = "utoipa", schema(write_only))]
    pub name: String,

    #[cfg_attr(feature = "utoipa", schema(write_only, required = false))]
    pub description: Option<String>,
}

/// An update to a room
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomPatch {
    #[cfg_attr(feature = "utoipa", schema(write_only))]
    pub name: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(write_only))]
    pub description: Option<Option<String>>,
}

// #[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
// #[sqlx(type_name = "room_type")]
// pub enum RoomType {
// 	Default,
// }
