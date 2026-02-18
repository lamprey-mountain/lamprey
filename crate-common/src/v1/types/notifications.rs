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
/// a notification; a unit of stuff that may show up in your inbox or be pushed to you
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Notification2 {
    pub id: NotificationId,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: Notification2Type,

    /// when this was added to the inbox
    pub added_at: Time,

    /// when this was read
    pub read_at: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum Notification2Type {
    /// someone sent a message you should look at
    Message {
        /// the channel this message was sent in
        channel_id: ChannelId,

        /// the id of the message that was sent
        message_id: MessageId,
    },

    /// someone reacted to a message you sent
    Reaction {
        /// the channel this message was sent in
        channel_id: ChannelId,

        /// the id of the message that was sent
        message_id: MessageId,
        // TODO: user id, reaction key
        // NOTE: i should probably aggregate all notifications into one bundle
    },
    // in the future, there'll probably be calendar events, document mentions, broadcast/voice activity, etc
}

// TODO: remove
/// a notification; a unit of stuff that may show up in your inbox or be pushed to you
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Notification {
    pub id: NotificationId,

    /// the channel this message was sent in
    pub channel_id: ChannelId,

    /// the id of the message that was sent
    pub message_id: MessageId,

    /// why this was created
    pub reason: NotificationReason,

    /// when this was added to the inbox
    pub added_at: Time,

    /// when this was read
    pub read_at: Option<Time>,
}

// TODO: remove
// in order of precedence
/// what caused this notification to be created
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum NotificationReason {
    /// user manually added this notification
    Reminder,

    /// this message mentioned you. overrides MentionBulk
    Mention,

    /// this message mentioned @room, @thread, or roles
    MentionBulk,

    /// this message replied to one of your own messages
    Reply,
    // /// this is a new thread
    // ThreadNew,

    // /// this thread is unread
    // /// message_id wont have any meaning, client should fetch context instead
    // ThreadUnread,
}

/// query your inbox
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct InboxListParams {
    /// only include notifications from these rooms
    #[cfg_attr(feature = "serde", serde(default))]
    #[schema(required = false, min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32))]
    pub room_id: Vec<RoomId>,

    /// only include notifications from these channels
    #[cfg_attr(feature = "serde", serde(default))]
    #[schema(required = false, min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32))]
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
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub message_ids: Vec<MessageId>,

    /// mark everything in these threads as read
    #[cfg_attr(feature = "serde", serde(default))]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub channel_ids: Vec<ChannelId>,

    /// mark everything in these rooms as read
    #[cfg_attr(feature = "serde", serde(default))]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
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
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub message_ids: Option<Vec<MessageId>>,

    /// restrict to just these channels
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub channel_ids: Option<Vec<ChannelId>>,

    /// restrict to just these rooms
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
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

impl Notification2 {
    /// get the tag for this notification
    ///
    /// notifications with the same tag will be deduplicated
    pub fn tag_id(&self) -> Uuid {
        match &self.ty {
            Notification2Type::Message { message_id, .. } => **message_id,
            Notification2Type::Reaction { message_id, .. } => **message_id,
        }
    }
}
