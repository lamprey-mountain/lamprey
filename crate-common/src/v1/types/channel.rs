#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::tag::Tag;
use crate::v1::types::user_config::UserConfigChannel;
use crate::v1::types::util::{some_option, Time};
use crate::v1::types::{util::Diff, ChannelVerId, PermissionOverwrite};
use crate::v1::types::{MediaId, MessageVerId, TagId, ThreadMember, User};

use super::{ChannelId, RoomId, UserId};

/// A channel
// TODO(#878): minimal data for channels
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

    // TODO: maybe rename this to topic? since most other platforms call it topic.
    // i guess i could also have topic and description be separate (short vs long)
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub description: Option<String>,

    /// url that this info channel should link to
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(url, length(min = 1, max = 2048)))]
    pub url: Option<String>,

    /// type specific data for this channel
    #[serde(rename = "type")]
    pub ty: ChannelType,

    /// number of people in this room
    pub member_count: u64,

    /// number of people who are online in this room
    pub online_count: u64,

    /// tags that are applied to this thread
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub tags: Option<Vec<TagId>>,

    /// the tags that are available in this forum. exists on Forum channels only.
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub tags_available: Option<Vec<Tag>>,

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

    /// whether users without ThreadManage can add other members to this thread
    #[serde(default)]
    pub invitable: bool,

    /// The user's thread member object, if the channel is a thread and the user is a member.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_member: Option<Box<ThreadMember>>,

    /// when to automatically archive this thread due to inactivity, in seconds
    pub auto_archive_duration: Option<u64>,

    /// the default auto archive duration in seconds to copy to threads created in this channel
    pub default_auto_archive_duration: Option<u64>,

    /// minimum delay in seconds between creating new threads
    // can only be set on channels with has_threads
    // must have ChannelManage/ThreadManage to change
    pub slowmode_thread: Option<u64>,

    /// minimum delay in seconds between creating new messages
    // can only be set on channels with has_text
    // must have ChannelManage/ThreadManage to change
    pub slowmode_message: Option<u64>,

    /// default slowmode_message for new threads
    ///
    /// this value is copied, changing this wont change old threads
    // can only be set on channels with has_threads
    // must have ChannelManage/ThreadManage to change
    pub default_slowmode_message: Option<u64>,

    /// when the current user can create a new thread
    pub slowmode_thread_expire_at: Option<Time>,

    /// when the current user can create a new message
    pub slowmode_message_expire_at: Option<Time>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ChannelType {
    /// instant messaging
    #[default]
    Text,

    /// announcement channel (like Text but with announcement-specific features)
    Announcement,

    /// a thread visible to anyone who can see the channel
    ThreadPublic,

    /// a thread that is only visible to thread members
    ThreadPrivate,

    /// a thread used in forums, behaving identically to ThreadPublic
    ThreadForum2,

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

    /// broadcast voice channel for many listeners
    Broadcast,

    /// category for grouping channels together
    Category,

    /// a calendar
    Calendar,

    /// experimental tree style long form chat history (like reddit or hackernews)
    Forum2,

    /// info channel without text/voice/threads
    Info,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, format = Uri, max_length = 1, min_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(url, length(min = 1, max = 2048)))]
    pub url: Option<String>,

    pub icon: Option<MediaId>,

    /// The type of this channel
    #[serde(default, rename = "type")]
    pub ty: ChannelType,

    /// tags to apply to this thread (overwrite, not append)
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
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
    #[serde(default)]
    pub permission_overwrites: Vec<PermissionOverwrite>,

    /// whether users without ThreadManage can add other members to this thread
    #[serde(default)]
    pub invitable: bool,

    pub auto_archive_duration: Option<u64>,

    pub default_auto_archive_duration: Option<u64>,

    pub slowmode_thread: Option<u64>,

    pub slowmode_message: Option<u64>,

    pub default_slowmode_message: Option<u64>,
}

// TODO(#874) split out channel create structs
#[cfg(any())]
mod split_channel_types {
    use super::{ChannelId, UserId};
    use crate::v1::types::PermissionOverwrite;
    use crate::v1::types::{ChannelType, MediaId, MessageCreate, TagId};

    // channel create room (do i allow creating threads with this endpoint?)
    pub struct ChannelCreateRoom {
        pub name: String,
        pub description: Option<String>,
        // // room channels can't have icons (yet?)
        // pub icon: Option<MediaId>,
        pub ty: ChannelType,
        pub nsfw: bool,
        pub bitrate: Option<u64>,
        pub user_limit: Option<u64>,
        pub parent_id: Option<ChannelId>,
        pub permission_overwrites: Vec<PermissionOverwrite>,
        pub auto_archive_duration: Option<u64>,
        pub default_auto_archive_duration: Option<u64>,
        pub slowmode_thread: Option<u64>,
        pub slowmode_message: Option<u64>,
        pub default_slowmode_message: Option<u64>,
    }

    // channel create dm
    pub struct ChannelCreateDm {
        pub name: String,
        pub description: Option<String>,
        pub icon: Option<MediaId>,
        /// must be Dm or Gdm
        pub ty: ChannelType,
        pub recipients: Option<Vec<UserId>>,
    }

    // thread create
    // maybe have a separate ThreadCreateForum type too?
    pub struct ThreadCreate {
        pub name: String,
        pub description: Option<String>,
        /// must be ThreadPublic or ThreadPrivate. must be ThreadPublic in forums (remove?)
        pub ty: ChannelType,
        /// tags to apply, only usable in forums
        pub tags: Option<Vec<TagId>>,
        /// the initial message for this thread, required in forums
        pub starter_message: Option<MessageCreate>,
        pub invitable: bool,
        pub auto_archive_duration: Option<u64>,
        pub slowmode_message: Option<u64>,
    }

    // thread create from message
    pub struct ThreadCreateFromMessage {
        pub name: String,
        pub description: Option<String>,
        /// must be ThreadPublic (remove in this case)
        pub ty: ChannelType,
        pub auto_archive_duration: Option<u64>,
        pub slowmode_message: Option<u64>,
    }

    struct ThreadCreateMaybeUnused {
        // // inherits from parent
        // pub nsfw: bool,
        // // maybe include this as users to include by default?
        // pub recipients: Option<Vec<UserId>>,
        // // exists as route param
        // pub parent_id: Option<ChannelId>,
    }
}

// unlikely to be used
#[cfg(any())]
mod granular_channel_data {
    use crate::v1::types::PermissionOverwrite;

    pub struct Channel {
        voice: Option<ChannelVoice>,
        thread: Option<ChannelThread>,
        threadable: Option<ChannelThreadable>,
        text: Option<ChannelText>,
        room: Option<ChannelRoom>,
    }

    pub struct ChannelVoice {
        pub bitrate: Option<u64>,
        pub user_limit: Option<u64>,
    }

    pub struct ChannelThread {
        pub auto_archive_duration: Option<u64>,
    }

    pub struct ChannelThreadable {
        pub default_auto_archive_duration: Option<u64>,
        pub slowmode_thread: Option<u64>,
        pub default_slowmode_message: Option<u64>,
    }

    pub struct ChannelText {
        pub slowmode_message: Option<u64>,
    }

    // for top level room channels
    pub struct ChannelRoom {
        pub nsfw: bool,
        pub permission_overwrites: Vec<PermissionOverwrite>,
    }
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

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, format = Uri, max_length = 1, min_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(url, length(min = 1, max = 2048)))]
    #[serde(default, deserialize_with = "some_option")]
    pub url: Option<Option<String>>,

    #[serde(default, deserialize_with = "some_option")]
    pub icon: Option<Option<MediaId>>,

    /// tags to apply to this thread (overwrite, not append)
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
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

    pub ty: Option<ChannelType>,

    #[serde(default, deserialize_with = "some_option")]
    pub parent_id: Option<Option<ChannelId>>,

    pub archived: Option<bool>,
    pub locked: Option<bool>,

    pub invitable: Option<bool>,

    #[serde(default, deserialize_with = "some_option")]
    pub auto_archive_duration: Option<Option<u64>>,

    #[serde(default, deserialize_with = "some_option")]
    pub default_auto_archive_duration: Option<Option<u64>>,

    #[serde(default, deserialize_with = "some_option")]
    pub slowmode_thread: Option<Option<u64>>,

    #[serde(default, deserialize_with = "some_option")]
    pub slowmode_message: Option<Option<u64>>,

    #[serde(default, deserialize_with = "some_option")]
    pub default_slowmode_message: Option<Option<u64>>,
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
            || self.url.changes(&other.url)
            || self.icon.changes(&other.icon)
            || self.tags.changes(&other.tags)
            || self.nsfw.changes(&other.nsfw)
            || self.bitrate.changes(&other.bitrate)
            || self.user_limit.changes(&other.user_limit)
            || self.owner_id.changes(&other.owner_id)
            || self.ty.changes(&other.ty)
            || self.parent_id.changes(&other.parent_id)
            || self.locked.changes(&other.locked)
            || self.invitable.changes(&other.invitable)
            || self
                .archived
                .is_some_and(|a| a != other.archived_at.is_some())
            || self
                .auto_archive_duration
                .changes(&other.auto_archive_duration)
            || self
                .default_auto_archive_duration
                .changes(&other.default_auto_archive_duration)
            || self.slowmode_thread.changes(&other.slowmode_thread)
            || self.slowmode_message.changes(&other.slowmode_message)
            || self
                .default_slowmode_message
                .changes(&other.default_slowmode_message)
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

impl ChannelType {
    pub fn is_thread(&self) -> bool {
        matches!(self, ChannelType::ThreadPublic | ChannelType::ThreadPrivate | ChannelType::ThreadForum2)
    }

    pub fn has_members(&self) -> bool {
        matches!(
            self,
            ChannelType::ThreadPublic | ChannelType::ThreadPrivate | ChannelType::Gdm
        )
    }

    pub fn has_text(&self) -> bool {
        matches!(
            self,
            ChannelType::ThreadPublic
                | ChannelType::ThreadPrivate
                | ChannelType::ThreadForum2
                | ChannelType::Text
                | ChannelType::Announcement
                | ChannelType::Dm
                | ChannelType::Gdm
                | ChannelType::Voice
                | ChannelType::Broadcast
        )
    }

    /// whether public threads can be created inside this channel
    pub fn has_public_threads(&self) -> bool {
        matches!(
            self,
            ChannelType::Text
                | ChannelType::Announcement
                | ChannelType::Dm
                | ChannelType::Gdm
                | ChannelType::Forum
                | ChannelType::Forum2
        )
    }

    /// whether private threads can be created inside this channel
    pub fn has_private_threads(&self) -> bool {
        matches!(self, ChannelType::Text | ChannelType::Dm | ChannelType::Gdm)
    }

    pub fn has_threads(&self) -> bool {
        self.has_public_threads() || self.has_private_threads()
    }

    pub fn has_voice(&self) -> bool {
        matches!(self, ChannelType::Voice | ChannelType::Broadcast)
    }

    pub fn has_url(&self) -> bool {
        matches!(self, ChannelType::Info)
    }

    /// for a thread to be taggable, it must be in a channel with has_tags
    pub fn is_taggable(&self) -> bool {
        matches!(self, ChannelType::ThreadPublic | ChannelType::ThreadPrivate | ChannelType::ThreadForum2)
    }

    pub fn has_tags(&self) -> bool {
        matches!(self, ChannelType::Forum | ChannelType::Forum2)
    }

    pub fn has_icon(&self) -> bool {
        matches!(self, ChannelType::Gdm)
    }

    /// if voice connections in this channel act like calls
    pub fn has_call(&self) -> bool {
        matches!(self, ChannelType::Dm | ChannelType::Gdm)
    }

    pub fn has_calendar(&self) -> bool {
        matches!(self, ChannelType::Calendar)
    }

    pub fn can_change_to(self, other: ChannelType) -> bool {
        match (self, other) {
            (ChannelType::ThreadPublic, ChannelType::ThreadPrivate) => true,
            (ChannelType::ThreadPrivate, ChannelType::ThreadPublic) => true,
            (ChannelType::Text, ChannelType::Announcement) => true,
            (ChannelType::Announcement, ChannelType::Text) => true,
            _ => false,
        }
    }

    /// if the member list subscription logic should restrict the list to thread members (instead of filtering all room members)
    pub fn member_list_uses_thread_members(&self) -> bool {
        matches!(self, ChannelType::Dm | ChannelType::Gdm) || self.is_thread()
    }
}
