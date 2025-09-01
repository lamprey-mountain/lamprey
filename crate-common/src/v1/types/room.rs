use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    notifications::NotifsRoom,
    util::{some_option, Diff},
    MediaId, Permission, UserId,
};

use super::{ids::RoomId, util::Time};

/// A room
// chose this name arbitrarily, maybe should be renamed to something like channel
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Room {
    /// A unique identifier for this room
    pub id: RoomId,

    /// A monotonically increasing id that is updated every time this room is modified.
    pub version_id: Uuid,

    /// The owner of this room. Owners have full admin permissions which cannot be revoked.
    ///
    /// This almost always exists, but for legacy rooms may be null
    pub owner_id: Option<UserId>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    // TODO: rename to `topic`
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    pub icon: Option<MediaId>,

    #[serde(flatten)]
    pub room_type: RoomType,

    /// number of people in this room
    pub member_count: u64,

    /// number of people who are online in this room
    pub online_count: u64,

    /// number of active threads
    pub thread_count: u64,

    // rooms can't be outright deleted, but some people might want to "clean up"
    // or "close" old rooms. archiving could be a good way to do that.
    pub archived_at: Option<Time>,

    /// anyone can view and join
    pub public: bool,
}

/// User-specific room data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomPrivate {
    pub notifications: NotifsRoom,
    /// resolved notifications for you
    pub permissions: Vec<Permission>,
}

/// Data required to create a room
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomCreate {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    pub icon: Option<MediaId>,
    pub public: Option<bool>,
}

/// An update to a room
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomPatch {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,

    pub icon: Option<Option<MediaId>>,
    pub public: Option<bool>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum RoomType {
    /// the default generic room type
    #[default]
    Default,

    #[deprecated]
    /// direct messages between two people
    Dm { participants: (UserId, UserId) },

    #[deprecated]
    /// system messages
    // or maybe these are dms from a System user
    System,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomMetrics {
    /// Total number of threads in this room (including archived or removed ones).
    pub thread_count: u64,

    /// Number of active threads in this room (excluding archived or removed ones).
    pub active_thread_count: u64,

    /// Total number of messages across all active threads in this room (excluding removed messages).
    pub message_count: u64,

    /// Total number of members in this room.
    pub member_count: u64,

    /// Total number of attachments from messages in this room.
    pub media_count: u64,

    /// Combined size (in bytes) of all attachments in this room.
    pub media_size: u64,
}

impl Diff<Room> for RoomPatch {
    fn changes(&self, other: &Room) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.icon.changes(&other.icon)
            || self.public.changes(&other.public)
    }
}
