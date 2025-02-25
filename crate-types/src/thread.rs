use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::util::some_option;
use crate::{util::Diff, ThreadVerId};
use crate::{CallId, MessageVerId};

use super::{RoomId, ThreadId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Thread {
    pub id: ThreadId,
    pub room_id: RoomId,
    pub creator_id: UserId,
    pub version_id: ThreadVerId,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub description: Option<String>,

    pub state: ThreadState,
    pub visibility: ThreadVisibility,

    #[serde(flatten)]
    pub info: ThreadInfo,
    // pub icon: Option<Media>,
    // do i use TagId or Tag?
    // pub tags: Vec<Tag>,
    // pub is_tag_required: bool,
    // pub member_count: u64,
    // pub online_count: u64,
    // pub state_updated_at: time::OffsetDateTime,
    // pub default_order: ThreadsOrder,
    // pub default_layout: ThreadsLayout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadCreateRequest {
    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, max_length = 1, min_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub description: Option<String>,
    // pub icon: Option<Media>,
    // pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ThreadPatch {
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

    #[serde(flatten)]
    pub state: Option<ThreadState>,
}

/// lifecycle of a thread
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "state")]
pub enum ThreadState {
    /// always remains active
    Pinned { pin_order: u32 },

    /// default state that new threads are in
    Active,

    /// goes straight to Deleted instead of Archived
    Temporary,

    /// inactive
    Archived,

    /// will be permanently deleted soon, visible to moderators
    Deleted,
}

/// who can view this thread
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ThreadVisibility {
    /// Everyone in the room can view
    // maybe use Room(RoomId) instead?,
    Room,
    // /// anyone in the room with a direct link can view
    // UnlistedRoom,

    // /// anyone can view
    // Unlisted,

    // /// anyone can find
    // Discoverable,

    // /// only visible to existing thread members
    // Private,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum ThreadInfo {
    /// instant messaging
    Chat {
        is_unread: bool,
        last_version_id: MessageVerId,
        last_read_id: Option<MessageVerId>,
        message_count: u64,
    },
    // /// linear chat history, similar to github/forgejo issues
    // ForumLinear(ThreadInfoChat),

    // /// tree-style chat history
    // ForumTree(ThreadInfoChat),

    // /// call
    // Voice(ThreadInfoVoice),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadInfoChat {
    pub last_version_id: MessageVerId,
    pub message_count: u64,
    // /// if this should be treated as an announcement
    // // TODO: define what an announcement thread does
    // pub is_announcement: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadInfoChatPrivate {
    pub is_unread: bool,
    pub last_read_id: Option<MessageVerId>,
    pub mention_count: u64,
    // pub notifications: NotificationConfigThread,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadInfoVoice {
    pub call_id: Option<CallId>,
    pub bitrate: u64,
    pub user_limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadInfoVoicePrivate {
    // what to put here?
}

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

impl Diff<Thread> for ThreadPatch {
    fn changes(&self, other: &Thread) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.state.changes(&other.state)
    }
}

impl Diff<Thread> for ThreadState {
    fn changes(&self, other: &Thread) -> bool {
        self != &other.state
    }
}

impl ThreadState {
    pub fn can_change_to(&self, _to: &ThreadState) -> bool {
        !matches!(self, Self::Deleted)
    }
}
