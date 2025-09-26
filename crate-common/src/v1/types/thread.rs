use serde::{Deserialize, Serialize};

pub use chat::{ThreadTypeChatPrivate, ThreadTypeChatPublic};
pub use forum::ThreadTypeForumTreePublic as ThreadTypeForumPublic;
pub use ThreadTypeChatPrivate as ThreadTypeForumPrivate;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::notifications::NotifsThread;
use crate::v1::types::util::{some_option, Time};
use crate::v1::types::{util::Diff, PermissionOverwrite, ThreadVerId};
use crate::v1::types::{MessageVerId, TagId, User};

use super::{RoomId, ThreadId, UserId};

pub mod chat;
pub mod forum;
pub mod voice;

// TODO: remove
#[cfg(feature = "feat_thread_type_event")]
pub mod event;

// TODO: remove
#[cfg(feature = "feat_thread_type_document")]
pub mod document;

// TODO: remove
#[cfg(feature = "feat_thread_type_table")]
pub mod table;

// TODO: remove, will re-add if needed later
#[cfg(feature = "feat_thread_type_report")]
pub mod report;

#[cfg(feature = "feat_thread_type_event")]
use event::{ThreadTypeEventPrivate, ThreadTypeEventPublic};

#[cfg(feature = "feat_thread_type_document")]
use document::{ThreadTypeDocumentPrivate, ThreadTypeDocumentPublic};

#[cfg(feature = "feat_thread_type_table")]
use table::{ThreadTypeTablePrivate, ThreadTypeTablePublic};

#[cfg(feature = "feat_thread_type_report")]
use report::{ThreadTypeReportPrivate, ThreadTypeReportPublic};

/// A thread
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Thread {
    pub id: ThreadId,
    pub room_id: Option<RoomId>,
    pub creator_id: UserId,

    /// only updates when the thread itself is updated, not the stuff in the thread
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

    /// type specific data for this thread
    #[serde(rename = "type")]
    pub ty: ThreadType,

    /// number of people in this room
    /// does not not update with ThreadSync
    pub member_count: u64,

    /// number of people who are online in this room
    /// does not not update with ThreadSync
    pub online_count: u64,

    // TODO(#72): tags
    /// tags that are applied to this thread
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub tags: Vec<TagId>,

    // TODO: rename to removed_at
    pub deleted_at: Option<Time>,
    pub archived_at: Option<Time>,

    /// a locked thread can only be interacted with (sending messages,
    /// (un)archiving, etc) by people with the `ThreadLock` permission
    pub locked: bool,

    /// the category thread this thread is in, if any
    pub parent_id: Option<ThreadId>,

    /// the position of this thread in the navbar
    ///
    /// - lower numbers come first (0 is the first thread)
    /// - threads with the same position are tiebroken by id
    /// - threads without a position come last, ordered by newest first
    pub position: Option<u16>,

    /// permission overwrites for this thread
    pub permission_overwrites: Vec<PermissionOverwrite>,

    /// not safe for work
    pub nsfw: bool,

    pub last_version_id: Option<MessageVerId>,
    pub message_count: Option<u64>,
    pub root_message_count: Option<u64>,

    /// bitrate, for voice thread. defaults to 65535 (64Kibps).
    #[cfg_attr(feature = "validator", validate(range(min = 8192)))]
    pub bitrate: Option<u64>,

    /// maximum number of users who can be in this voice thread
    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    pub user_limit: Option<u64>,

    // private (TODO: maybe move these into a `private` field with their own struct?)
    pub is_unread: Option<bool>,
    pub last_read_id: Option<MessageVerId>,
    pub mention_count: Option<u64>,
    // being able to have an exact unread count would be nice, but would be hard
    // to implement efficiently. if someone marks a very old message as unread,
    // i don't want to hang while counting potentially thousands of messages!
    // pub unread_count: u64,
    pub notifications: Option<NotifsThread>, // TODO: remove
    // pub user_config: UserConfigThread,
    /// for dm threads, this is who the dm is with
    /// DEPRECATED: use `recipients` instead
    #[cfg_attr(feature = "utoipa", schema(deprecated))]
    pub recipient: Option<User>,

    /// for dm threads, this is who the dm is with
    pub recipients: Vec<User>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ThreadType {
    /// instant messaging
    #[default]
    Chat,

    /// instant messaging direct message
    Dm,

    /// instant messaging group direct message
    Gdm,

    #[cfg(feature = "feat_thread_type_forums")]
    /// long form chat history
    Forum,

    /// call
    Voice,

    /// category for grouping threads together
    Category,

    #[cfg(feature = "feat_thread_type_event")]
    /// event
    // seems surprisingly hard to get right
    Event,

    #[cfg(feature = "feat_thread_type_document")]
    /// document
    // maybe some crdt document/wiki page...?
    // another far future thing that needs design
    Document,

    #[cfg(feature = "feat_thread_type_table")]
    // arbitrary data storage? like a spreadsheet or database table?
    Table,

    #[cfg(feature = "feat_thread_type_report")]
    Report,
    // Calendar,
    // Files,
    // Redex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "validator", derive(Validate))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadCreate {
    #[cfg_attr(feature = "utoipa", schema(max_length = 1, min_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, max_length = 1, min_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    pub description: Option<String>,
    // TODO: icons for gdm threads
    // pub icon: Option<Media>,
    /// The type of this thread
    #[serde(default, rename = "type")]
    pub ty: ThreadType,

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
    // /// the initial message for this thread
    // pub starter_message: MessageCreate,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// reorder some threads
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ThreadReorder {
    /// the threads to reorder
    #[serde(default)]
    #[validate(length(min = 1, max = 1024))]
    pub threads: Vec<ThreadReorderItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ThreadReorderItem {
    pub id: ThreadId,

    #[serde(default, deserialize_with = "some_option")]
    pub position: Option<Option<u16>>,

    #[serde(default, deserialize_with = "some_option")]
    pub parent_id: Option<Option<ThreadId>>,
}

impl Diff<Thread> for ThreadPatch {
    fn changes(&self, other: &Thread) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.tags.changes(&other.tags)
            || self.nsfw.changes(&other.nsfw)
            || self.bitrate.changes(&other.bitrate)
            || self.user_limit.changes(&other.user_limit)
    }
}

impl Thread {
    /// remove private user data
    pub fn strip(self) -> Thread {
        Thread {
            is_unread: None,
            last_read_id: None,
            mention_count: None,
            notifications: None,
            ..self
        }
    }
}

impl ThreadPatch {
    pub fn minimal_for(self, other: &Thread) -> ThreadPatch {
        ThreadPatch {
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
        }
    }
}
