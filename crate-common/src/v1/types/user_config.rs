//! things that the user can configure

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::notifications::NotifsGlobal;

// use crate::v1::types::notifications::NotifsGlobal;

// #[cfg(feature = "validator")]
// use validator::Validate;

/// configuration for a user
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct UserConfig {
    // TODO: implement notifications
    /// global notification config
    pub notifs: NotifsGlobal,

    // TODO: implement privacy/safety stuff
    // /// privacy and safety config
    // pub privacy_safety: PrivacySafety,
    /// config specific to frontend
    pub frontend: HashMap<String, serde_json::Value>,
}

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct PrivacySafety {
//     pub friends: FriendsFilter,
//     pub rooms: HashMap<RoomId, UserRoomConfig>,

//     /// copied, not inherited
//     pub rooms_default: UserRoomConfig,

//     // for dms
//     // pub spam_filtering: none | mild | aggressive,
//     // pub nsfw_filtering: none | non-friends | everyone,
// }

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct UserRoomConfig {
//     /// whether to allow direct messages
//     /// bots and moderators can always dm you
//     pub allow_dms: bool,

//     /// whether to strip location metadata (exif)
//     pub strip_location: bool,
// }

// /// who can send friend requests
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct FriendsFilter {
//     /// overrides everything if you need a break from all the friends
//     pub pause_until: Option<Time>,

//     /// allow everyone to send you a friend request
//     /// overrides everything else
//     pub allow_everyone: bool,

//     /// allow everyone who shares a room with you send you a friend request
//     /// requires the room to have allow_dms set
//     pub allow_mutual_room: bool,

//     /// allow everyone who shares a friend with you send you a friend request
//     pub allow_mutual_friend: bool,
// }
