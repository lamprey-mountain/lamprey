#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use uuid::Uuid;
#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    reaction::ReactionKeyParam, util::Time, Channel, ChannelId, Message, MessageId, NotificationId,
    Room, RoomId, UserId,
};

pub mod bytes;
pub mod preferences;

// TODO: maybe include a `completed_at` field if this action is "completable"?
/// a notification; a unit of stuff that may show up in your inbox or be pushed to you
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Notification {
    pub id: NotificationId,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: NotificationType,

    /// when this was added to the inbox
    pub added_at: Time,

    /// when this was read
    pub read_at: Option<Time>,

    /// user defined note for this notification
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotificationType {
    /// someone sent a message you should look at
    Message {
        /// the room this message was sent in
        room_id: Option<RoomId>,

        /// the channel this message was sent in
        channel_id: ChannelId,

        /// the id of the message that was sent
        message_id: MessageId,

        /// the author of this message
        user_id: UserId,

        /// this notification was triggered by an @user
        mention_user: bool,

        /// this notification was triggered by an @everyone or @here mention
        mention_everyone: bool,

        /// this notification was triggered by a @role mention
        mention_role: bool,

        /// this notification was triggered by a reply
        reply: bool,
    },

    /// someone reacted to a message you sent
    Reaction {
        /// the room this reaction was sent in
        room_id: Option<RoomId>,

        /// the channel this reaction was sent in
        channel_id: ChannelId,

        /// the id of the message that was reacted to
        message_id: MessageId,

        /// the user who created this reaction
        user_id: UserId,

        reaction_key: ReactionKeyParam,
    },

    /// a thread was created
    Thread {
        /// the room this thread was created in
        room_id: Option<RoomId>,

        /// the id of the thread
        thread_id: ChannelId,

        /// the user who created this thread
        user_id: UserId,
    },

    /// you sent a friend request
    FriendRequestSent { user_id: UserId },

    /// someone sent a friend request to you
    FriendRequestReceived { user_id: UserId },

    /// someone accepted your friend request or you accepted someone's friend request
    FriendRequestAccepted { user_id: UserId },
    // TODO: calendar events, document mentions, broadcast/voice activity, etc
}

/// query your inbox
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct InboxListParams {
    /// only include notifications from these rooms
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 32)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
    pub room_id: Vec<RoomId>,

    /// only include notifications from these channels
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 32)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
    pub channel_id: Vec<ChannelId>,

    /// include messages marked as read too
    #[cfg_attr(feature = "serde", serde(default))]
    pub include_read: bool,
}

/// create a new message reminder notification
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotificationCreate {
    /// the channel this message was sent in
    pub channel_id: ChannelId,

    /// the id of the message that was sent
    pub message_id: MessageId,

    /// set this in the future to create a reminder
    pub added_at: Option<Time>,
}

/// mark some notifications as read (or unread)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct NotificationMarkRead {
    /// mark these messages as read
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 1024)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub message_ids: Vec<MessageId>,

    /// mark everything in these threads as read
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 1024)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub channel_ids: Vec<ChannelId>,

    /// mark everything in these rooms as read
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 1024)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub room_ids: Vec<RoomId>,

    /// mark everything as read
    #[cfg_attr(feature = "serde", serde(default))]
    pub everything: bool,
}

/// delete some notifications
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct NotificationFlush {
    /// restrict to just notifications before (including) this message id
    pub before: Option<MessageId>,

    /// restrict to just notifications after (including) this message id
    pub after: Option<MessageId>,

    /// restrict to just these messages
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 1024)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub message_ids: Option<Vec<MessageId>>,

    /// restrict to just these channels
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 1024)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub channel_ids: Option<Vec<ChannelId>>,

    /// restrict to just these rooms
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 1024)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 1024)))]
    pub room_ids: Option<Vec<RoomId>>,

    /// also include unread notifications
    #[cfg_attr(feature = "serde", serde(default))]
    pub include_unread: bool,
}

/// paginate through your notifications
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct NotificationPagination {
    pub notifications: Vec<Notification>,
    pub total: u64,
    pub has_more: bool,
    pub cursor: Option<String>,

    // extra context
    pub channels: Vec<Channel>,
    pub messages: Vec<Message>,
    pub rooms: Vec<Room>,
    // TODO: add room members, thread members, users
}

// TODO: move this to Notification or move methods in Notification to NotificationType
impl NotificationType {
    pub fn message_id(&self) -> Option<MessageId> {
        match self {
            NotificationType::Message { message_id, .. } => Some(*message_id),
            NotificationType::Reaction { message_id, .. } => Some(*message_id),
            NotificationType::Thread { .. } => None,
            NotificationType::FriendRequestSent { .. } => None,
            NotificationType::FriendRequestReceived { .. } => None,
            NotificationType::FriendRequestAccepted { .. } => None,
        }
    }
}

impl Notification {
    /// get the tag for this notification
    ///
    /// notifications with the same tag will be deduplicated
    // TODO: also deduplicate in inbox?
    pub fn tag_id(&self) -> Uuid {
        match &self.ty {
            NotificationType::Message { message_id, .. } => **message_id,
            NotificationType::Reaction { message_id, .. } => **message_id,
            NotificationType::Thread { thread_id, .. } => **thread_id,
            NotificationType::FriendRequestSent { user_id } => **user_id,
            NotificationType::FriendRequestReceived { user_id } => **user_id,
            NotificationType::FriendRequestAccepted { user_id } => **user_id,
        }
    }

    pub fn channel_id(&self) -> Option<ChannelId> {
        match &self.ty {
            NotificationType::Message { channel_id, .. } => Some(*channel_id),
            NotificationType::Reaction { channel_id, .. } => Some(*channel_id),
            NotificationType::Thread { thread_id, .. } => Some(*thread_id),
            NotificationType::FriendRequestSent { .. } => None,
            NotificationType::FriendRequestReceived { .. } => None,
            NotificationType::FriendRequestAccepted { .. } => None,
        }
    }

    pub fn room_id(&self) -> Option<RoomId> {
        match &self.ty {
            NotificationType::Message { room_id, .. } => *room_id,
            NotificationType::Reaction { room_id, .. } => *room_id,
            NotificationType::Thread { room_id, .. } => *room_id,
            NotificationType::FriendRequestSent { .. } => None,
            NotificationType::FriendRequestReceived { .. } => None,
            NotificationType::FriendRequestAccepted { .. } => None,
        }
    }

    pub fn message_id(&self) -> Option<MessageId> {
        self.ty.message_id()
    }
}
