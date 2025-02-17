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
    pub id: RoomId,

    /// A monotonically increasing id that is updated every time this room is modified.
    pub version_id: Uuid,

    pub name: String,

    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub description: Option<String>,
    // pub room_type: RoomType,
}

/// Data required to create a room
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomCreate {
    pub name: String,

    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub description: Option<String>,
}

/// An update to a room
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomPatch {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,
}

use crate::util::some_option;
// enum RoomType {
//     Default,
//     Dm { other: User },
//     Reports,
// }

// #[derive(Debug, Clone, PartialEq, Eq, ToSchema, Serialize, Deserialize, sqlx::Type)]
// #[sqlx(type_name = "room_type")]
// pub enum RoomType {
// 	Default,
// 	Dm,
// }
