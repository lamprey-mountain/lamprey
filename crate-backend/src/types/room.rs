use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::ids::RoomId;

/// A room
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
pub struct Room {
    /// A unique identifier for this room
    #[schema(read_only)]
    pub id: RoomId,

    /// A monotonically increasing id that is updated every time this room is modified.
    #[schema(read_only)]
    pub version_id: Uuid,

    #[schema(read_only)]
    pub name: String,

    #[schema(read_only, required = false)]
    pub description: Option<String>,
    // default_roles: Vec<RoleId>,
}

/// Data required to create a room
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct RoomCreate {
    #[schema(write_only)]
    pub name: String,

    #[schema(write_only, required = false)]
    pub description: Option<String>,
}

/// An update to a room
#[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct RoomPatch {
    #[schema(write_only)]
    pub name: Option<String>,

    #[schema(write_only)]
    pub description: Option<Option<String>>,
}

// #[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
// #[sqlx(type_name = "room_type")]
// pub enum RoomType {
// 	Default,
// }
