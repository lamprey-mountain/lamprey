use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use super::{
    channel::ChannelCreate,
    ids::{ChannelId, RoomId},
    role::RoleCreate,
    user::User,
    util::Time,
    PaginationKey,
};

/// a short, unique identifier for a room template.
#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomTemplateCode(pub String);

impl fmt::Display for RoomTemplateCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PaginationKey for RoomTemplateCode {
    fn min() -> Self {
        RoomTemplateCode("".to_string())
    }

    fn max() -> Self {
        // This is just a random long string, assuming codes are alphanumeric.
        RoomTemplateCode("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_string())
    }
}

/// A template for creating rooms.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomTemplate {
    /// unique identifier for this template
    pub code: RoomTemplateCode,

    /// name for this template
    pub name: String,
    pub description: String,
    pub created_at: Time,

    /// updated whenever template is edited or synced
    pub updated_at: Time,

    /// user who created this template
    pub creator: User,

    // only returned for the creator
    /// the room this template was created from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_room_id: Option<RoomId>,

    // only returned for the creator
    /// if the source room and the template have diverged
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirty: Option<bool>,

    pub snapshot: RoomTemplateSnapshot,
}

/// a snapshot of a room
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomTemplateSnapshot {
    pub channels: Vec<RoomTemplateChannel>,
    pub roles: Vec<RoomTemplateRole>,
    pub welcome_channel_id: Option<ChannelId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomTemplateCreate {
    // user must be able to view the room
    pub room_id: RoomId,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomTemplatePatch {
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomTemplateChannel {
    #[serde(flatten)]
    pub inner: ChannelCreate,

    /// temporary placeholder id, for use in parent_id
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomTemplateRole {
    #[serde(flatten)]
    pub inner: RoleCreate,

    /// temporary placeholder id, for use in permission overwrites
    pub id: Uuid,
}
