//! things that the user can configure
// TODO: strongly type user settings

use std::collections::HashMap;

use lamprey_macros::record;

use crate::v1::types::{
    misc::Time,
    notifications::preferences::{NotifsChannel, NotifsGlobal, NotifsRoom},
};

pub mod room_sidebar;

/// preferences for a user
#[record]
#[derive(Default)]
pub struct PreferencesGlobal {
    /// global notification config
    pub notifs: NotifsGlobal,

    /// global privacy settings
    pub privacy: PreferencesGlobalPrivacy,

    /// config specific to frontend
    pub frontend: PreferencesGlobalFrontend,
}

#[record]
#[derive(Default)]
pub struct PreferencesGlobalFrontend {
    /// room navigation sidebar
    pub room_sidebar: room_sidebar::Sidebar,

    /// extra implementation defined config
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// preferences for a user in a room
#[record]
#[derive(Default)]
pub struct PreferencesRoom {
    /// room notification config
    pub notifs: NotifsRoom,

    /// room privacy settings
    pub privacy: PreferencesRoomPrivacy,

    /// config specific to frontend
    pub frontend: PreferencesRoomFrontend,
}

#[record]
#[derive(Default)]
pub struct PreferencesRoomFrontend {
    /// extra implementation defined config
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// preferences for a user in a channel or thread
#[record]
#[derive(Default)]
pub struct PreferencesChannel {
    /// thread notification config
    pub notifs: NotifsChannel,

    /// config specific to frontend
    pub frontend: PreferencesChannelFrontend,
}

#[record]
#[derive(Default)]
pub struct PreferencesChannelFrontend {
    /// extra implementation defined config
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// preferences for a user for another user
#[record]
#[derive(Default)]
pub struct PreferencesUser {
    /// config in voice threads
    pub voice: VoiceConfig,

    /// config specific to frontend
    pub frontend: PreferencesUserFrontend,
}

#[record]
#[derive(Default)]
pub struct PreferencesUserFrontend {
    /// extra implementation defined config
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// voice config the local user can set on someone else
#[record]
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

/// who can send friend requests
#[record]
pub struct PreferencesGlobalFriends {
    /// pause all friend requests
    ///
    /// overrides everything else
    pub pause_until: Option<Time>,

    /// allow everyone to send you a friend request
    ///
    /// overrides everything except pause_until
    pub allow_everyone: bool,

    /// allow everyone who shares a room with you send you a friend request
    ///
    /// requires the room to have allow_dms set
    pub allow_mutual_room: bool,

    /// allow everyone who shares a friend with you send you a friend request
    pub allow_mutual_friend: bool,
}

impl Default for PreferencesGlobalFriends {
    fn default() -> Self {
        Self {
            pause_until: None,
            allow_everyone: true, // NOTE: probably should disable this if a lot of people join
            allow_mutual_room: true,
            allow_mutual_friend: true,
        }
    }
}

/// user privacy settings globally
#[record]
pub struct PreferencesGlobalPrivacy {
    pub friends: PreferencesGlobalFriends,

    /// default dms config for new rooms
    ///
    /// copied, not inherited
    pub dms: bool,

    /// default rpc config for new rooms
    ///
    /// copied, not inherited
    pub rpc: bool,

    /// default exif config for new rooms
    ///
    /// copied, not inherited
    pub exif: bool,
}

impl Default for PreferencesGlobalPrivacy {
    fn default() -> Self {
        // NOTE: maybe i should make it more or less permissive depending on if the room is public
        Self {
            friends: Default::default(),
            dms: true,
            rpc: true,
            exif: true,
        }
    }
}

/// user privacy settings for a room
#[record]
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

impl Default for PreferencesRoomPrivacy {
    fn default() -> Self {
        Self {
            dms: true,
            friends: true,
            rpc: true,
            exif: true,
        }
    }
}
