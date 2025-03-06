use serde::{Deserialize, Serialize};

use text::{ThreadTypeChatPrivate, ThreadTypeChatPublic};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::util::{some_option, Time};
use crate::{util::Diff, ThreadVerId};

use super::{RoomId, ThreadId, UserId};

pub mod text;

#[cfg(feature = "feat_thread_type_voice")]
pub mod voice;

#[cfg(feature = "feat_thread_type_event")]
pub mod event;

#[cfg(feature = "feat_thread_type_document")]
pub mod document;

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
    pub state_updated_at: Time,

    /// who can see this thread
    pub visibility: ThreadVisibility,

    /// type specific data for this thread
    #[serde(flatten)]
    pub info: ThreadPublic,

    /// user-specific data for this thread
    /// this should be the same type as info
    // i couldn't figure out how to get bootleg dependent types to work in rust, so eh
    #[serde(flatten)]
    pub private: Option<ThreadPrivate>,

    /// number of people in this room
    pub member_count: u64,

    /// number of people who are online in this room
    pub online_count: u64,
    // TODO(#72): tags
    // #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
    // pub tags: Vec<TagId>,

    // pub link: Vec<ThreadLink>, // probably will limit the number of links
    // pub forward: Option<ThreadForward>,

    // // this is something i've been wondering about for a while
    // // `locked` would be easier to implement and could have custom acls, but
    // // it might add extra complexity (it's an extra thing that can affect
    // // auth that doesn't use the "standard" roles system)
    // // alternative would be to let moderators edit permissions for threads
    // // TODO(#243): implement this. it makes life much easier.
    // pub locked: bool,

    // /// if this should be treated as an announcement
    // // TODO: define what an announcement thread does
    // // pretty much copy/forward the thread to any following rooms
    // // (is it a copy or reference? ie. does each followed room get its
    // // own discussion thread or is there one big discussion thread shared
    // // everywhere? the latter sounds like it could be extremely painful,
    // // but maybe i could do both. create a new copy for every follower,
    // // and include a link to the source.)
    // pub is_announcement: bool,
}

/// type-specific data for threads
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[derive(strum::EnumDiscriminants)]
// #[strum_discriminants(vis(pub), name(ThreadType))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum ThreadPublic {
    /// instant messaging
    Chat(ThreadTypeChatPublic),

    #[cfg(feature = "feat_thread_type_forums")]
    /// linear long form chat history, similar to github/forgejo issues
    // TODO: come up with a less terrible name
    ForumLinear(ThreadTypeChatPublic),

    #[cfg(feature = "feat_thread_type_forums")]
    /// tree-style chat history
    ForumTree(ThreadTypeChatPublic),

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
    Table(()),
}

/// user-specific data for threads
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum ThreadPrivate {
    /// instant messaging
    Chat(ThreadTypeChatPrivate),

    #[cfg(feature = "feat_thread_type_forums")]
    /// linear long form chat history, similar to github/forgejo issues
    ForumLinear(ThreadTypeChatPrivate),

    #[cfg(feature = "feat_thread_type_forums")]
    /// tree-style chat history
    ForumTree(ThreadTypeChatPrivate),

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
    Table(()),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ThreadType {
    /// instant messaging
    #[default]
    Chat,

    #[cfg(feature = "feat_thread_type_forums")]
    /// linear long form chat history, similar to github/forgejo issues
    // TODO: come up with a less terrible name
    ForumLinear,

    #[cfg(feature = "feat_thread_type_forums")]
    /// tree-style chat history
    ForumTree,

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
    // /// tags to apply to this thread
    // #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
    // pub tags: Vec<TagId>,
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
    // /// tags to apply to this thread
    // #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
    // pub tags: Vec<TagId>,
}

/// lifecycle of a thread
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    // /// exists but is hidden from the main list/timeline
    // Removed,
    /// will be permanently deleted soon, visible to moderators
    Deleted,
}

/// who can view this thread
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ThreadVisibility {
    /// Inherit visibility from this (the parent) room
    // maybe use Room(RoomId) instead?,
    Room,

    /// anyone can view
    Public {
        /// anyone can search for and find this; otherwise, this is unlisted
        is_discoverable: bool,

        /// whether anyone can join without an invite; otherwise, this is view only
        is_free_for_all: bool,
    },

    /// only visible to existing thread members
    Private {
        // anything here?
    },
}

#[cfg(feature = "feat_thread_linking")]
pub mod thread_linking {
    use crate::{util::Time, ThreadId, UserId};

    // need a way to define access control for linking threads
    // linked threads need to be in the same room
    pub struct ThreadLink {
        pub thread_id: ThreadId,
        #[serde(flatten)]
        pub info: ThreadLinkInfo,
        pub reason: Option<String>,
        /// None if automated
        pub by: Option<UserId>,
        pub at: Time,
    }

    pub enum ThreadLinkInfo {
        /// discussion, comments, calls
        /// what if there are lots of threads? eg. a thread for every suggestion in a document?
        /// maybe also need a way to hide threads with certain links
        Discussion,

        /// show a button/link to view this other thread instead of this one
        /// (maybe redirect automatically in some places?)
        // (stolen from irc)
        Forward,

        /// Forward + special handling? (eg. search in both threads by default)
        Duplicate,

        /// the source announcement thread
        Announcement,

        /// a child thread (creates a hierarchy). possibly unnecessary, might be
        /// useful for Voice threads.
        Child,

        // not sure about these "generic relations/links"
        /// generic unidirectional relationship (source)
        Incoming,

        /// generic unidirectional relationship (target)
        Outgoing,

        /// generic bidirectional relationship
        Related,
    }
}

#[cfg(feature = "feat_thread_linking")]
use thread_linking::*;

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
