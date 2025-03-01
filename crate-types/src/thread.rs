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
    // pub link: Vec<ThreadLink>, // probably will limit the number of links
    // pub forward: Option<ThreadForward>,

    // // this is something i've been wondering about for a while
    // // `locked` would be easier to implement and could have custom acls, but
    // // it might add extra complexity (it's an extra thing that can affect
    // // auth that doesn't use the "standard" roles system)
    // // alternative would be to let moderators edit permissions for threads,
    // pub locked: bool,
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
    // // for Event threads
    // // special case of Archived? maybe have something like is_cancelled for archived?
    // // might be good to be able to have something like github's "closed as not planned"
    // // alternatively, i could use special purpose tags
    // Cancelled,
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

    // /// event
    // // seems surprisingly hard to get right
    // Event(ThreadInfoEvent),

    // /// document
    // // maybe some crdt document/wiki page...?
    // // another far future thing that needs design
    // Document(ThreadInfoDocument),
}

// /// tell everyone viewing this thread to go to another thread (maybe redirect automatically in some places?)
// // (stolen from irc)
// struct ThreadForward {
//     thread_id: ThreadId,
//     reason: Option<String>,
//     forwarded_by: UserId,
//     forwarded_at: Time,
// }

// need a way to define access control for linking threads
// linked threads need to be in the same room
// pub struct ThreadLink {
//     pub thread_id: ThreadId,
//     pub link_type: ThreadLinkType,
//     pub purpose: ThreadLinkPurpose,
// }

// pub enum ThreadLinkType {
//     Outgoing,
//     Incoming,
//     Bidirectional,
// }

// pub enum ThreadLinkPurpose {
//     /// ThreadInfoChat threads -> any thread (eg. commenting)
//     Discussion,
//
//     /// any thread -> ThreadInfoVoice threads
//     Call,
//
//     /// any thread <-> any thread
//     Related,
//
//     /// any thread -> any thread
//     // maybe have some special handling? (eg. make searches return messages in both threads)
//     Duplicate,
//
//     // what else?
// }

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub enum ThreadInfoEventLocation {
//     Geo(crate::media::Location),
//     Url(url::Url),
// }

// // probably need a better repr
// // doesn't need to be fully vanilla cron, can be more typesafe/user friendly if needed
// // or use lib.rs/cron
// // also figure out how i18n works for other calendar systems
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct CronStr(String);

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct Cron {
//     minutes: Vec<CronValue<60>>,
//     hours: Vec<CronValue<60>>,
//     days: Vec<CronValue<31>>,
//     months: Vec<CronValue<12>>,
//     year: Vec<CronValue<12>>,
//     days_of_week: Vec<CronValue<7>>,
// }

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub enum CronValue<const MAX: u8> {
//     All,
//     Single(u8),
//     Range(u8, u8),
// }

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub enum TimeGranularity {
//     Day,
//     Hour,
//     Minute,
// }

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub enum EventDuration {
//     AllDay,
//     Minutes(u64),
// }

// // could be part of ThreadState? unsure how to do this appropriately though
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// enum EventStatus {
//     Scheduled,
//     Active,
//     // maybe these two are the same as archived
//     Finished,
//     Cancelled {
//         cancelled_reason: Text,
//         cancelled_at: Time,
//         cancelled_by: UserId,
//     },
// }

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// pub struct ThreadInfoEvent {
//     pub name: Text,
//     pub description: Text,
//     pub color: Color,
//     pub icon: Media,
//     pub banner: Media,

//     pub location: ThreadInfoEventLocation,
//     pub url: Url,

//     pub time: Time,
//     pub timezone: Option<Timezone>, // maybe i want this in more fields?
//     pub time_granularity: TimeGranularity,
//     pub repeats: Cron,
//     pub until: Time,
//     pub duration: EventDuration,

//     pub user_limit: Option<u64>,
//     pub user_rsvp_yes: u64,
//     pub user_rsvp_no: u64,
//     pub user_rsvp_maybe: u64,
//     pub user_rsvp_invited: u64,
//     pub user_rsvp_waitlisted: u64,
//     pub autofill_waitlist: bool,
//     pub status: EventStatus,
// }

// // could be extension of ThreadMembership
// enum EventRsvpType {
//     Yes,
//     No,
//     Maybe,
//     Invited,
//     Waitlisted,
// }

// struct EventRsvp {
//     thread_id: ThreadId,
//     user_id: UserId,
//     status: EventRsvpType,
// }

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
