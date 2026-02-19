#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use uuid::Uuid;
#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{util::Time, Channel, ChannelId, MessageId, NotificationId, Room, RoomId};
use crate::v2::types::message::Message;

pub mod preferences;

// TODO: use this instead of the current notification type
// TODO: maybe include a `completed` field if this action is "completable"?
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum NotificationType {
    /// someone sent a message you should look at
    Message {
        /// the room this message was sent in
        room_id: RoomId,

        /// the channel this message was sent in
        channel_id: ChannelId,

        /// the id of the message that was sent
        message_id: MessageId,
    },

    /// someone reacted to a message you sent
    Reaction {
        /// the room this message was sent in
        room_id: RoomId,

        /// the channel this message was sent in
        channel_id: ChannelId,

        /// the id of the message that was sent
        message_id: MessageId,
        // TODO: user id, reaction key
        // NOTE: i should probably aggregate all notifications into one bundle
    },
    // in the future, there'll probably be calendar events, document mentions, broadcast/voice activity, etc
    // also FriendRequestReceived and FriendRequestAccepted (notif lifecycle? like one friend notif that gets updated over time?)
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

    pub channels: Vec<Channel>,
    pub messages: Vec<Message>,
    pub rooms: Vec<Room>,
}

// /// serialized notification payload, sent through web push
// // TODO: implement
// pub struct NotificationBytes {
//     // 1 byte: version
//     // 1 byte: type
//     // 2 bytes: 0x00 0x00 (unused, use for flags?)

//     // 4 bytes: notification id
//     // 4 bytes: channel id
//     // 4 bytes: message id

//     // flags: is edit, author is ignored, channel is muted, what else?
// }

impl NotificationType {
    pub fn message_id(&self) -> Option<MessageId> {
        match self {
            NotificationType::Message { message_id, .. } => Some(*message_id),
            NotificationType::Reaction { message_id, .. } => Some(*message_id),
        }
    }
}

impl Notification {
    /// get the tag for this notification
    ///
    /// notifications with the same tag will be deduplicated
    pub fn tag_id(&self) -> Uuid {
        match &self.ty {
            NotificationType::Message { message_id, .. } => **message_id,
            NotificationType::Reaction { message_id, .. } => **message_id,
        }
    }

    pub fn channel_id(&self) -> Option<ChannelId> {
        match &self.ty {
            NotificationType::Message { channel_id, .. } => Some(*channel_id),
            NotificationType::Reaction { channel_id, .. } => Some(*channel_id),
        }
    }

    pub fn room_id(&self) -> Option<RoomId> {
        match &self.ty {
            NotificationType::Message { room_id, .. } => Some(*room_id),
            NotificationType::Reaction { room_id, .. } => Some(*room_id),
        }
    }

    pub fn message_id(&self) -> Option<MessageId> {
        self.ty.message_id()
    }
}
