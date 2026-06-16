#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{
    v1::types::{
        ChannelSeq, ChannelType, ThreadMember,
        calendar::Calendar,
        document::{Document, Wiki},
        federation::Remote,
        misc::Time,
        preferences::PreferencesChannel,
        voice::{ChannelBroadcast, ChannelVoice},
    },
    v2::types::{ChannelId, ChannelVerId, MessageId, RoomId, UserId},
};

pub mod components;
pub mod create;
pub mod update;

// NOTE: reuse old types
pub use crate::v1::types::channel::{ChannelReorder, ChannelReorderItem, Locked};

pub use components::{
    ChannelDm, ChannelInfo, ChannelRoom, ChannelText, ChannelThread, ChannelThreaded,
};
pub use create::{ChannelCreateDm, ChannelCreateRoom, ThreadCreate, ThreadCreateFromMessage};
pub use update::ChannelUpdate;

/// A channel
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Channel {
    pub id: ChannelId,
    pub version_id: ChannelVerId,
    pub room_id: Option<RoomId>,

    /// creator of the channel or thread
    pub creator_id: UserId,

    /// owner of the group dm
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub owner_id: Option<UserId>,

    /// monotonic sync sequence number, incremented on every action.
    ///
    /// used for incremental channel sync.
    #[cfg_attr(feature = "serde", serde(default))]
    pub seq: ChannelSeq,

    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub ty: ChannelType,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub remote: Option<Remote<ChannelId>>,

    // --- shared data ---
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub name: String,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 2048)))]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub description: Option<String>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub preferences: Option<Box<PreferencesChannel>>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub removed_at: Option<Time>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub archived_at: Option<Time>,

    /// number of people who can view this channel
    pub member_count: u64,

    /// number of people who can view this channel and are online
    pub online_count: u64,

    // --- channel components ---
    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub document: Option<Box<Document>>,

    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub wiki: Option<Box<Wiki>>,

    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub calendar: Option<Box<Calendar>>,

    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub voice: Option<Box<ChannelVoice>>,

    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub broadcast: Option<Box<ChannelBroadcast>>,

    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub text: Option<Box<ChannelText>>,

    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub threaded: Option<Box<ChannelThreaded>>,

    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub thread: Option<Box<ChannelThread>>,

    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub info: Option<Box<ChannelInfo>>,

    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub room: Option<Box<ChannelRoom>>,

    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub dm: Option<Box<ChannelDm>>,
    // TODO: redex: ChannelRedex?
}

/// channel data private to a user
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelPrivate {
    pub read_marker_id: Option<MessageId>,
    pub mention_count: Option<u64>,
    pub preferences: Option<Box<PreferencesChannel>>,

    /// when the current user can create a new message
    pub slowmode_message_expire_at: Option<Time>,

    /// when the current user can create a new thread
    pub slowmode_thread_expire_at: Option<Time>,

    /// The user's thread member object, if the channel is a thread and the user is a member.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub thread_member: Option<Box<ThreadMember>>,
}

impl Channel {
    /// remove all user-specific data
    pub fn strip_private(&mut self) {
        self.preferences = None;
        if let Some(text) = &mut self.text {
            text.strip_private();
        }
        if let Some(threaded) = &mut self.threaded {
            threaded.strip_private();
        }
        if let Some(thread) = &mut self.thread {
            thread.strip_private();
        }
    }

    /// merge in user-specific data
    pub fn merge_private(&mut self, private: ChannelPrivate) {
        self.preferences = private.preferences;
        if let Some(text) = &mut self.text {
            text.read_marker_id = private.read_marker_id;
            text.mention_count = private.mention_count;
            text.slowmode_message_expire_at = private.slowmode_message_expire_at;
        }
        if let Some(threaded) = &mut self.threaded {
            threaded.slowmode_thread_expire_at = private.slowmode_thread_expire_at;
        }
        if let Some(thread) = &mut self.thread {
            thread.thread_member = private.thread_member;
        }
    }
}
