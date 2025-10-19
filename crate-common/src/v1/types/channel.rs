use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::user_config::UserConfigChannel;
use crate::v1::types::util::{some_option, Time};
use crate::v1::types::{util::Diff, ChannelVerId, PermissionOverwrite};
use crate::v1::types::{MediaId, MessageVerId, TagId, User};

use super::{ChannelId, RoomId, UserId};

/// A channel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Channel {
    pub id: ChannelId,
    pub room_id: Option<RoomId>,
    pub creator_id: UserId,

    /// owner of the group dm
    pub owner_id: Option<UserId>,

    /// only updates when the channel itself is updated, not the stuff in the channel
    pub version_id: ChannelVerId,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub description: Option<String>,

    /// type specific data for this channel
    #[serde(rename = "type")]
    pub ty: ChannelType,

    /// number of people in this room
    pub member_count: u64,

    /// number of people who are online in this room
    pub online_count: u64,

    // TODO(#72): tags
    /// tags that are applied to this thread
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub tags: Vec<TagId>,

    // TODO: rename to removed_at
    pub deleted_at: Option<Time>,
    pub archived_at: Option<Time>,

    /// a locked channel can only be interacted with (sending messages,
    /// (un)archiving, etc) by people with the `ThreadLock` permission
    pub locked: bool,

    /// the channel this channel is in, if any
    pub parent_id: Option<ChannelId>,

    /// the position of this channel in the navbar
    ///
    /// - lower numbers come first (0 is the first channel)
    /// - channels with the same position are tiebroken by id
    /// - channels without a position come last, ordered by newest first
    pub position: Option<u16>,

    /// permission overwrites for this channel
    pub permission_overwrites: Vec<PermissionOverwrite>,

    /// not safe for work
    pub nsfw: bool,

    pub last_version_id: Option<MessageVerId>,
    pub message_count: Option<u64>,
    pub root_message_count: Option<u64>,

    /// bitrate, for voice channels. defaults to 65535 (64Kibps).
    #[cfg_attr(feature = "validator", validate(range(min = 8192)))]
    pub bitrate: Option<u64>,

    /// maximum number of users who can be in this voice channel
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    pub user_limit: Option<u64>,

    pub is_unread: Option<bool>,
    pub last_read_id: Option<MessageVerId>,
    pub mention_count: Option<u64>,
    pub user_config: Option<UserConfigChannel>,

    /// for dm and gdm channels, this is who the dm is with
    pub recipients: Vec<User>,

    /// for gdm channels, a custom icon
    pub icon: Option<MediaId>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ChannelType {
    /// instant messaging
    #[default]
    Text,

    /// a thread visible to anyone who can see the channel
    ThreadPublic,

    /// a thread that is only visible to thread members
    ThreadPrivate,

    /// instant messaging direct message
    Dm,

    /// instant messaging group direct message
    Gdm,

    #[cfg(feature = "feat_thread_type_forums")]
    /// long form chat history
    // NOTE: this will be redone later. Forum will be the type of the parent channel, internal threads will use ThreadFoo channels.
    Forum,

    /// a call
    Voice,

    /// category for grouping channels together
    Category,

    /// a calendar
    Calendar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelCreate {
    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, max_length = 1, min_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub description: Option<String>,

    pub icon: Option<MediaId>,

    /// The type of this channel
    #[serde(default, rename = "type")]
    pub ty: ChannelType,

    /// tags to apply to this thread (overwrite, not append)
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4096)))]
    pub tags: Option<Vec<TagId>>,

    /// not safe for work
    #[serde(default)]
    pub nsfw: bool,

    /// the recipient(s) for this dm/gdm
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 10)))]
    pub recipients: Option<Vec<UserId>>,

    #[cfg_attr(feature = "validator", validate(range(min = 8192)))]
    pub bitrate: Option<u64>,

    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    pub user_limit: Option<u64>,

    // required for threads
    pub parent_id: Option<ChannelId>,
    // /// the initial message for this thread
    // pub starter_message: MessageCreate,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelPatch {
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 64)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 2048))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,

    #[serde(default, deserialize_with = "some_option")]
    pub icon: Option<Option<MediaId>>,

    /// tags to apply to this thread (overwrite, not append)
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4096)))]
    pub tags: Option<Vec<TagId>>,

    /// not safe for work
    pub nsfw: Option<bool>,

    #[cfg_attr(feature = "validator", validate(range(min = 8192)))]
    #[serde(default, deserialize_with = "some_option")]
    pub bitrate: Option<Option<u64>>,

    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    #[serde(default, deserialize_with = "some_option")]
    pub user_limit: Option<Option<u64>>,

    #[serde(default, deserialize_with = "some_option")]
    pub owner_id: Option<Option<UserId>>,
}

/// reorder some channels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelReorder {
    /// the channels to reorder
    #[serde(default)]
    #[validate(length(min = 1, max = 1024))]
    pub channels: Vec<ChannelReorderItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelReorderItem {
    pub id: ChannelId,

    #[serde(default, deserialize_with = "some_option")]
    pub position: Option<Option<u16>>,

    #[serde(default, deserialize_with = "some_option")]
    pub parent_id: Option<Option<ChannelId>>,
}

impl Diff<Channel> for ChannelPatch {
    fn changes(&self, other: &Channel) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.icon.changes(&other.icon)
            || self.tags.changes(&other.tags)
            || self.nsfw.changes(&other.nsfw)
            || self.bitrate.changes(&other.bitrate)
            || self.user_limit.changes(&other.user_limit)
            || self.owner_id.changes(&other.owner_id)
    }
}

impl Channel {
    /// remove private user data
    pub fn strip(self) -> Channel {
        Channel {
            is_unread: None,
            last_read_id: None,
            mention_count: None,
            user_config: None,
            ..self
        }
    }
}

impl ChannelPatch {
    pub fn minimal_for(self, other: &Channel) -> ChannelPatch {
        ChannelPatch {
            name: if self.name.changes(&other.name) {
                self.name
            } else {
                None
            },
            description: if self.description.changes(&other.description) {
                self.description
            } else {
                None
            },
            icon: if self.icon.changes(&other.icon) {
                self.icon
            } else {
                None
            },
            tags: if self.tags.changes(&other.tags) {
                self.tags
            } else {
                None
            },
            nsfw: if self.nsfw.changes(&other.nsfw) {
                self.nsfw
            } else {
                None
            },
            bitrate: if self.bitrate.changes(&other.bitrate) {
                self.bitrate
            } else {
                None
            },
            user_limit: if self.user_limit.changes(&other.user_limit) {
                self.user_limit
            } else {
                None
            },
            owner_id: if self.owner_id.changes(&other.owner_id) {
                self.owner_id
            } else {
                None
            },
        }
    }
}

impl ChannelType {
    pub fn is_thread(&self) -> bool {
        matches!(self, ChannelType::ThreadPublic | ChannelType::ThreadPrivate)
    }
}
