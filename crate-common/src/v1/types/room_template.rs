use std::fmt;

use lamprey_macros::record;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{
    PaginationKey,
    channel::ChannelCreate,
    ids::{ChannelId, RoomId},
    role::RoleCreate,
    user::User,
    util::Time,
};

/// a short, unique identifier for a room template.
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[record]
pub struct RoomTemplate {
    /// unique identifier for this template
    pub code: RoomTemplateCode,

    /// name for this template
    pub name: String,
    pub description: String,

    /// when this template was created
    pub created_at: Time,

    /// when this template was last edited or synced
    pub updated_at: Time,

    /// the user who created this template
    pub creator: User,

    /// the room this template was created from
    ///
    /// returned for the creator and anyone who can view the room
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_room_id: Option<RoomId>,

    /// if the source room and the template have diverged
    ///
    /// only returned for the creator if they have `RoomEdit`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirty: Option<bool>,

    pub snapshot: RoomTemplateSnapshot,
    // // TODO: add?
    // /// the number of times this room template has been used
    // pub uses: u64,
}

#[record]
pub struct RoomTemplateCreate {
    /// the id of the room to turn into a template
    ///
    /// the caller must have the `RoomEdit` permission in the room
    // TODO(?): allow uploading RoomTemplateSnapshot directly without creating a room first? probably not, i dont want to recreate every endpoint on room for room templates.
    // though, maybe i could have a special RoomType::Template? has an associated room template, automatically syncs changes to template, not returned in ambient, ...?
    pub room_id: RoomId,

    #[validate(length(min = 1, max = 64))]
    pub name: String,

    #[validate(length(min = 1, max = 8192))]
    pub description: String,
}

#[record]
pub struct RoomTemplatePatch {
    #[validate(length(min = 1, max = 64))]
    pub name: Option<String>,

    #[validate(length(min = 1, max = 8192))]
    pub description: Option<String>,
}

/// a snapshot of a room
#[record]
pub struct RoomTemplateSnapshot {
    pub channels: Vec<RoomTemplateChannel>,
    pub roles: Vec<RoomTemplateRole>,
    pub welcome_channel_id: Option<ChannelId>,
    pub afk_channel_id: Option<ChannelId>,
    pub afk_channel_timeout: u64,
}

#[record]
pub struct RoomTemplateChannel {
    #[serde(flatten)]
    pub inner: ChannelCreate,

    /// temporary placeholder id, for use in parent_id
    pub id: Uuid,

    pub position: u16,
}

#[record]
pub struct RoomTemplateRole {
    #[serde(flatten)]
    pub inner: RoleCreate,

    /// placeholder id, for use in permission overwrites
    pub id: Uuid,

    pub position: u16,
}
