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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct UserConfig {
    /// global notification config
    pub notifs: NotifsGlobal,

    // TODO
    // /// privacy and safety config
    // pub privacy_safety: PrivacySafety,

    // /// feature flags
    // pub features: Vec<Feature>,
    /// config specific to frontend
    pub frontend: HashMap<String, serde_json::Value>,
}

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct PrivacySafety {
//     pub dm: DmsFilter,
//     pub friends: FriendsFilter,
// }

// /// who can send friend requests
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct FriendsFilter {
//     pub allow_everyone: bool,
//     pub allow_mutual_room: bool,
//     pub allow_mutual_friend: bool,
// }

// /// filtering for direct messages
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct DmsFilter {
//     pub allow_everyone: bool,
//     pub allow_nsfw_friend: bool,
//     pub allow_nsfw_everyone: bool,
// }
