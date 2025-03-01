use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{
    util::{some_option, Diff},
    UserId,
};

use super::ids::RoomId;

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

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub description: Option<String>,

    #[serde(rename = "type")]
    pub room_type: RoomType,
    // pub visibility: RoomVisibility,

    // pub member_count: u64,
    // pub online_count: u64,
    // pub views: RoomView,
}

// /// User-specific room data
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct RoomPrivate {
//     pub notifications: NotificationConfigRoom,
// }

// a room represents a single logical system of access control (members,
// roles, etc) but people might want to have "multiple rooms". a roomview would
// essentially be a (search? tag?) filter displayed as a separate "place".
//
// the reasons why this should exist pretty much boil down to how the ui
// is designed. depending on how i design everything, this might not even be
// necessary.
//
// struct RoomView {
//     filter: (),
// }

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
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RoomType {
    /// the default generic room type
    #[default]
    Default,

    /// direct messages between two people
    Dm { participants: (UserId, UserId) },

    /// for reports
    Reports { report: crate::moderation::Report },

    /// system messages
    // or maybe these are dms from a System user
    System,
}

/// who can view this room
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RoomVisibility {
    /// invite only
    #[default]
    Private,

    /// anyone can view
    Unlisted {
        /// whether anyone can join or if they still need an invite
        anyone_can_join: bool,
    },

    /// anyone can find
    Discoverable {
        /// whether anyone can join or if they still need an invite
        anyone_can_join: bool,
    },
}

// unsure how these should work
// struct SystemMessages {
//     user_join: SystemMessagesTarget,
//     moderation_report: SystemMessagesTarget,
// }

// enum SystemMessagesTarget {
//     Create,
//     Reuse(ThreadId),
// }

impl Diff<Room> for RoomPatch {
    fn changes(&self, other: &Room) -> bool {
        self.name.changes(&other.name) || self.description.changes(&other.description)
    }
}
