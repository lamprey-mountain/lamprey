use serde::{Deserialize, Serialize};

pub use chat::{ThreadTypeChatPrivate, ThreadTypeChatPublic};
pub use forum::ThreadTypeForumTreePublic as ThreadTypeForumPublic;
pub use ThreadTypeChatPrivate as ThreadTypeForumPrivate;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::util::{some_option, Time};
use crate::v1::types::TagId;
use crate::v1::types::{util::Diff, PermissionOverwrite, ThreadVerId};

use super::{RoomId, ThreadId, UserId};

pub mod chat;
pub mod forum;

#[cfg(feature = "feat_thread_type_voice")]
pub mod voice;

#[cfg(feature = "feat_thread_type_event")]
pub mod event;

#[cfg(feature = "feat_thread_type_document")]
pub mod document;

#[cfg(feature = "feat_thread_type_table")]
pub mod table;

#[cfg(feature = "feat_thread_type_report")]
pub mod report;

#[cfg(feature = "feat_thread_type_voice")]
use voice::{ThreadTypeVoicePrivate, ThreadTypeVoicePublic};

#[cfg(feature = "feat_thread_type_event")]
use event::{ThreadTypeEventPrivate, ThreadTypeEventPublic};

#[cfg(feature = "feat_thread_type_document")]
use document::{ThreadTypeDocumentPrivate, ThreadTypeDocumentPublic};

#[cfg(feature = "feat_thread_type_table")]
use table::{ThreadTypeTablePrivate, ThreadTypeTablePublic};

#[cfg(feature = "feat_thread_type_report")]
use report::{ThreadTypeReportPrivate, ThreadTypeReportPublic};

/// A thread
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(flatten)]
    pub info: ThreadPublic,

    /// user-specific data for this thread
    /// this should be the same type as info
    // i couldn't figure out how to get bootleg dependent types to work in rust, so eh
    #[serde(flatten)]
    pub private: Option<ThreadPrivate>,

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

    pub deleted_at: Option<Time>,
    pub archived_at: Option<Time>,
    pub locked_at: Option<Time>,

    /// permission overwrites for this thread
    pub permission_overwrites: Vec<PermissionOverwrite>,

    /// not safe for work
    pub nsfw: bool,
}

/// type-specific data for threads
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[derive(strum::EnumDiscriminants)]
// #[strum_discriminants(vis(pub), name(ThreadType))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum ThreadPublic {
    /// instant messaging
    Chat(ThreadTypeChatPublic),

    /// instant messaging direct message
    Dm(ThreadTypeChatPublic),

    #[cfg(feature = "feat_thread_type_forums")]
    /// long form chat history
    Forum(ThreadTypeForumPublic),

    #[cfg(feature = "feat_thread_type_voice")]
    /// call
    Voice(ThreadTypeVoicePublic),

    #[cfg(feature = "feat_thread_type_event")]
    /// event
    // seems surprisingly hard to get right
    Event(ThreadTypeEventPublic),

    #[cfg(feature = "feat_thread_type_document")]
    /// document
    // maybe some crdt document/wiki page...?
    // another far future thing that needs design
    Document(ThreadTypeDocumentPublic),

    #[cfg(feature = "feat_thread_type_table")]
    // arbitrary data storage? like a spreadsheet or database table?
    Table(ThreadTypeTablePublic),

    #[cfg(feature = "feat_thread_type_report")]
    Report(ThreadTypeReportPublic),
}

/// user-specific data for threads
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum ThreadPrivate {
    /// instant messaging
    Chat(ThreadTypeChatPrivate),

    /// instant messaging direct message
    Dm(ThreadTypeChatPrivate),

    #[cfg(feature = "feat_thread_type_forums")]
    /// long form chat history
    Forum(ThreadTypeForumPrivate),

    #[cfg(feature = "feat_thread_type_voice")]
    /// call
    Voice(ThreadTypeVoicePrivate),

    #[cfg(feature = "feat_thread_type_event")]
    /// event
    // seems surprisingly hard to get right
    Event(ThreadTypeEventPrivate),

    #[cfg(feature = "feat_thread_type_document")]
    /// document
    // maybe some crdt document/wiki page...?
    // another far future thing that needs design
    Document(ThreadTypeDocumentPrivate),

    #[cfg(feature = "feat_thread_type_table")]
    // arbitrary data storage? like a spreadsheet or database table?
    Table(ThreadTypeTablePrivate),

    #[cfg(feature = "feat_thread_type_report")]
    Report(ThreadTypeReportPrivate),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[non_exhaustive]
pub enum ThreadType {
    /// instant messaging
    #[default]
    Chat,

    /// instant messaging direct message
    Dm,

    #[cfg(feature = "feat_thread_type_forums")]
    /// long form chat history
    Forum,

    #[cfg(feature = "feat_thread_type_voice")]
    /// call
    Voice,

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
}

impl Diff<Thread> for ThreadPatch {
    fn changes(&self, other: &Thread) -> bool {
        self.name.changes(&other.name)
            || self.description.changes(&other.description)
            || self.tags.changes(&other.tags)
            || self.nsfw.changes(&other.nsfw)
    }
}

impl Thread {
    /// remove private user data
    pub fn strip(self) -> Thread {
        Thread {
            private: None,
            ..self
        }
    }

    /// add private user data
    pub fn with_private(self, data: ThreadPrivate) -> Thread {
        Thread {
            private: Some(data),
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
        }
    }
}
