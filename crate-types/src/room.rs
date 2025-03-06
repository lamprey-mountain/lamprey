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

    #[serde(flatten)]
    pub room_type: RoomType,

    pub visibility: RoomVisibility,

    /// number of people in this room
    pub member_count: u64,

    /// number of people who are online in this room
    pub online_count: u64,

    /// number of active threads
    pub thread_count: u64,

    pub default_order: ThreadsOrder,
    pub default_layout: ThreadsLayout,
    // pub views: RoomView,
    // pub available_tags: Vec, // to be used in threads
    // pub applied_tags: Vec, // added to this room
    // pub language: Language,
}

// /// User-specific room data
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct RoomPrivate {
//     pub notifications: NotificationConfigRoom,
//     pub permissions: Vec<Permission>,
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
#[serde(tag = "type")]
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
    Public {
        /// anyone can search for and find this; otherwise, this is unlisted
        is_discoverable: bool,

        /// whether anyone can join without an invite; otherwise, this is view only
        is_free_for_all: bool,
    },
    // /// anyone can apply to join
    // Applicable {
    //     /// anyone can search for and find this; otherwise, this is unlisted
    //     is_discoverable: bool,
    //
    //     /// the application they have to fill out
    //     application: (),
    // },
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

/// how to sort the room's thread list
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ThreadsOrder {
    #[default]
    /// newest threads first
    Time,

    /// latest activity first
    Activity,
    // /// weights based on activity and time
    // Hot,

    // /// engagement causes ranking to *lower*
    // Cool,
    // // /// returns posts randomly!
    // // Shuffle,

    // // /// most of that specific reaction first
    // // Reactions(Emoji),

    // // theres probably a better way to do this
    // // Reverse(Box<ThreadsOrder>)
}

/// how to display the room's thread list
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ThreadsLayout {
    /// laid out in a list with each post as its own "card"; kind of like reddit
    #[default]
    Card,

    /// more compact, only shows thumbnails for media; kind of like old reddit
    Compact,

    /// media in a regularly sized grid; like imageboorus
    Gallery,

    /// media in a staggered grid; like tumblr or pintrist
    Masonry,
}

impl Diff<Room> for RoomPatch {
    fn changes(&self, other: &Room) -> bool {
        self.name.changes(&other.name) || self.description.changes(&other.description)
    }
}
