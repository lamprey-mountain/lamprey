//! things that the user can configure
// TODO: strongly type user settings

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// TODO: add sanity checks
// #[cfg(feature = "validator")]
// use validator::Validate;

use crate::v1::types::{
    misc::Time,
    notifications::preferences::{NotifsChannel, NotifsGlobal, NotifsRoom},
};

/// preferences for a user
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesGlobal {
    /// global notification config
    pub notifs: NotifsGlobal,

    /// global privacy settings
    pub privacy: PreferencesGlobalPrivacy,

    /// config specific to frontend
    pub frontend: PreferencesGlobalFrontend,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesGlobalFrontend {
    /// extra implementation defined config
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub extra: HashMap<String, serde_json::Value>,
}

/// preferences for a user in a room
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesRoom {
    /// room notification config
    pub notifs: NotifsRoom,

    /// room privacy settings
    pub privacy: PreferencesRoomPrivacy,

    /// config specific to frontend
    pub frontend: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesRoomFrontend {
    /// extra implementation defined config
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub extra: HashMap<String, serde_json::Value>,
}

/// preferences for a user in a thread
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesChannel {
    /// thread notification config
    pub notifs: NotifsChannel,

    /// config specific to frontend
    pub frontend: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesChannelFrontend {
    /// extra implementation defined config
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub extra: HashMap<String, serde_json::Value>,
}

/// preferences for a user for another user
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesUser {
    /// config in voice threads
    pub voice: VoiceConfig,

    /// config specific to frontend
    pub frontend: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesUserFrontend {
    /// extra implementation defined config
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub extra: HashMap<String, serde_json::Value>,
}

/// voice config the local user can set on someone else
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct VoiceConfig {
    /// whether to mute voice
    pub mute: bool,

    /// defaults to 1 (aka 100% volume)
    pub volume: f64,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            mute: false,
            volume: 1.0,
        }
    }
}

/// user privacy settings for friends
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesGlobalFriends {
    pub filter: FriendsFilter,
}

/// user privacy settings globally
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesGlobalPrivacy {
    pub friends: PreferencesGlobalFriends,

    /// default room privacy setings
    ///
    /// copied, not inherited
    pub rooms_default: PreferencesRoomPrivacy,
}

/// user privacy settings for a room
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct PreferencesRoomPrivacy {
    /// allow dms from room members
    ///
    /// bots, moderators, and friends can always start dms
    pub dms: bool,

    /// allow friend requests from mutual room members
    pub friends: bool,

    /// share rich presence with mutual room members
    pub rpc: bool,

    /// whether to enable exif metadata, including location.
    ///
    /// setting to false will strip sensitive exif data
    pub exif: bool,
}

/// who can send friend requests
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct FriendsFilter {
    /// pause all friend requests
    ///
    /// overrides everything else
    pub pause_until: Option<Time>,

    /// allow everyone to send you a friend request
    ///
    /// overrides everything except pause_until
    pub allow_everyone: bool,

    /// allow everyone who shares a room with you send you a friend request
    /// requires the room to have allow_dms set
    pub allow_mutual_room: bool,

    /// allow everyone who shares a friend with you send you a friend request
    pub allow_mutual_friend: bool,
}
