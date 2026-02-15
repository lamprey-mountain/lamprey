#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    notifications::NotifsRoom,
    user_config::PreferencesRoom,
    util::{some_option, Diff},
    ChannelId, MediaId, Permission, UserId,
};

use super::{ids::RoomId, util::Time};

/// A room is a collection of members and acls in the form of roles. Each room
/// has an audit log to log administrative actions.
///
/// Default rooms, which most people are concerned with, contain threads, emoji,
/// and so on for instant messaging.
// chose this name arbitrarily, maybe should be renamed to something else.
// discord uses "guild", maybe if i do domain-name-per-room "zone" could work...
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    pub icon: Option<MediaId>,

    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub banner: Option<MediaId>,

    #[serde(rename = "type")]
    pub room_type: RoomType,

    /// number of people in this room
    pub member_count: u64,

    /// number of people who are online in this room
    pub online_count: u64,

    /// number of active channels
    pub channel_count: u64,

    /// number of emojis in room
    // will be needed for virtual scrolling with unlimited emoji
    // TODO(#956): implement
    pub emoji_count: u64,

    // rooms can't be outright deleted, but some people might want to "clean up"
    // or "close" old rooms. archiving could be a good way to do that.
    pub archived_at: Option<Time>,

    /// anyone can view and join
    pub public: bool,

    /// where member join messages will be sent
    pub welcome_channel_id: Option<ChannelId>,

    /// whether this room is read-only. permissions for all room members (including owner) will be masked to View and ViewAuditLog, similar to timing out a single user.
    pub quarantined: bool,
    pub user_config: Option<PreferencesRoom>,

    /// security configuration for this room
    pub security: RoomSecurity,

    /// automatically move afk people to this channel
    pub afk_channel_id: Option<ChannelId>,

    /// how long to wait before moving idle people to the afk channel, in milliseconds
    pub afk_channel_timeout: u64,

    #[cfg(any())]
    pub remote: Option<Remote>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomSecurity {
    pub require_mfa: bool,
    pub require_sudo: bool,
}

// NOTE: may be removed later, i dont see that much of a reason for this
/// a minimal preview of a room
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomPreview {
    pub id: RoomId,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    pub description: Option<String>,

    pub icon: Option<MediaId>,

    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub banner: Option<MediaId>,

    /// number of people in this room
    pub member_count: u64,

    /// number of people who are online in this room
    pub online_count: u64,
}

/// User-specific room data
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomPrivate {
    pub notifications: NotifsRoom,
    /// resolved notifications for you
    pub permissions: Vec<Permission>,
}

/// Data required to create a room
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

    #[cfg_attr(feature = "utoipa", schema(required = false))]
    pub banner: Option<MediaId>,

    pub public: Option<bool>,
    // /// the template to create this room from
    // pub snapshot: Option<RoomTemplateSnapshot>,
}

/// An update to a room
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

    #[serde(default, deserialize_with = "some_option")]
    pub icon: Option<Option<MediaId>>,
    #[serde(default, deserialize_with = "some_option")]
    pub banner: Option<Option<MediaId>>,
    pub public: Option<bool>,

    /// where member join messages will be sent
    pub welcome_channel_id: Option<Option<ChannelId>>,
    /// automatically move afk people to this channel
    pub afk_channel_id: Option<Option<ChannelId>>,
    /// how long to wait before moving idle people to the afk channel, in milliseconds
    pub afk_channel_timeout: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct RoomSecurityUpdate {
    pub require_mfa: Option<bool>,
    pub require_sudo: Option<bool>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RoomType {
    /// the default generic room type
    #[default]
    Default,

    /// server pseudo room
    Server,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct TransferOwnership {
    pub owner_id: UserId,
}

// TODO: move
pub use super::search::RoomSearchRequest as SearchRooms;

impl Diff<Room> for RoomPatch {
    fn changes(&self, other: &Room) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.icon.changes(&other.icon)
            || self.banner.changes(&other.banner)
            || self.public.changes(&other.public)
            || self.welcome_channel_id.changes(&other.welcome_channel_id)
            || self.afk_channel_id.changes(&other.afk_channel_id)
            || self.afk_channel_timeout.changes(&other.afk_channel_timeout)
    }
}
