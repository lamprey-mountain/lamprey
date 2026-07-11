#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::{
    v1::types::{
        Locked, PermissionOverwrite, ThreadMember, User,
        channel::components::{ForumLayout, ForumSorting},
        misc::Time,
        tag::TagMinimal,
    },
    v2::types::{ChannelId, MediaId, MessageId},
};

/// a channel that messages can be sent in
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelText {
    /// minimum delay in seconds between creating new messages
    ///
    /// can only be set on channels with text. must have ChannelManage permission to change, or ThreadManage if this is a thread.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub slowmode_message: Option<u64>,

    /// when the current user can create a new message
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub slowmode_message_expire_at: Option<Time>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub read_marker_id: Option<MessageId>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub mention_count: Option<u64>,

    /// the id of the last message in this channel
    ///
    /// used for read marker handling
    pub last_message_id: Option<MessageId>,

    pub last_pin_timestamp: Option<Time>,

    pub message_count: u64,
    pub root_message_count: u64,
}

/// a channel that contains threads
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChannelThreaded {
    /// minimum delay in seconds between creating new threads
    ///
    /// can only be set on channels with has_threads. must have ChannelManage permission to change.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub slowmode_thread: Option<u64>,

    /// the default auto archive duration in seconds to copy to threads created in this channel
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub default_auto_archive_duration: Option<u64>,

    /// default slowmode_message for new threads
    ///
    /// this value is copied, changing this wont change old threads. can only be set on channels with has_threads. must have ChannelManage permission to change.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub default_slowmode_message: Option<u64>,

    /// when the current user can create a new thread
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub slowmode_thread_expire_at: Option<Time>,

    /// number of tags in this Forum, Forum2, or Ticket channel
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub tag_count: Option<u64>,

    pub sorting: ForumSorting,
    pub default_layout: ForumLayout,
}

/// a channel that is a thread
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelThread {
    /// when to automatically archive this thread due to inactivity, in seconds
    pub auto_archive_duration: Option<u64>,

    /// whether users without ThreadManage can add other members to this thread
    #[cfg_attr(feature = "serde", serde(default))]
    pub invitable: bool,

    /// The user's thread member object, if the channel is a thread and the user is a member.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub thread_member: Option<Box<ThreadMember>>,

    /// tags that are applied to this thread
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub tags: Option<Vec<TagMinimal>>,
}

/// an info channel
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelInfo {
    /// url that this info channel should link to
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 2048)
    )]
    #[cfg_attr(feature = "validator", validate(url, length(min = 1, max = 2048)))]
    pub url: Option<String>,
}

/// a channel that's in a room
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelRoom {
    /// the position of this channel in the navbar
    ///
    /// - lower numbers come first (0 is the first channel)
    /// - channels with the same position are tiebroken by id
    /// - channels without a position come last, ordered by newest first
    pub position: u16,

    /// permission overwrites for this channel
    // TODO: max length
    pub permission_overwrites: Vec<PermissionOverwrite>,

    /// not safe for work
    #[cfg_attr(feature = "serde", serde(default))]
    pub nsfw: bool,

    /// whether this channel is locked and has restricted permissions
    ///
    /// a locked channel can only be interacted with (sending messages,
    /// (un)archiving, etc) by anyone who has any of
    ///
    /// - a role in allowed_roles
    /// - the `ChannelManage` permission
    /// - the `ThreadLock` or `ThreadManage` permission IF this channel is a thread
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub locked: Option<Locked>,

    /// the channel this channel is in, if any
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub parent_id: Option<ChannelId>,
}

/// a dm or gdm channel
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelDm {
    /// for dm and gdm channels, this is who the dm is with
    #[cfg_attr(feature = "serde", serde(default))]
    pub recipients: Vec<User>,

    /// for gdm channels, a custom icon
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub icon: Option<MediaId>,
}

impl ChannelText {
    pub fn strip_private(&mut self) {
        self.slowmode_message_expire_at = None;
        self.read_marker_id = None;
        self.mention_count = None;
    }
}

impl ChannelThreaded {
    pub fn strip_private(&mut self) {
        self.slowmode_thread_expire_at = None;
    }
}

impl ChannelThread {
    pub fn strip_private(&mut self) {
        self.thread_member = None;
    }
}
