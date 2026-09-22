use lamprey_macros::record;

use uuid::Uuid;

use crate::v1::types::{
    Channel, ChannelId, Message, MessageId, NotificationId, RoomId, RoomMember, ThreadMember,
    User, UserId, reaction::ReactionKeyParam, util::Time,
};

pub mod bytes;
pub mod preferences;

// TODO: maybe include a `completed_at` field if this action is "completable"?
/// a notification
///
/// a unit of stuff that may show up in your inbox or be pushed to you
#[record]
pub struct Notification {
    pub id: NotificationId,

    #[serde(flatten)]
    pub ty: NotificationType,

    /// when this was added to the inbox
    pub added_at: Time,

    /// when this was read
    pub read_at: Option<Time>,

    /// user defined note for this notification
    pub note: Option<String>,
}

#[record]
#[serde(tag = "type")]
pub enum NotificationType {
    /// someone sent a message you should look at
    // TODO: make this a generic "Mention" notification type?
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
    // CalendarEventCreate,
    // CalendarEventUpdate,
    // CalendarEventMention, // create a channel for every calendar event
    // CalendarEventStarted, // only push this, don't save in inbox?
    // DocumentCreate, // same as Thread?
    // DocumentUpdate,
    // DocumentMention, // requires explicit call to document mention endpoint, backend checks if document actually contains mention
    // StreamStart, // someone started a stream
    // maybe a notif type for missed messages?
}

/// query your inbox
#[record(params)]
pub struct NotificationQuery {
    /// only include notifications from these rooms
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32))]
    pub room_id: Vec<RoomId>,

    /// only include notifications from these channels
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 32)]
    #[validate(length(min = 1, max = 32))]
    pub channel_id: Vec<ChannelId>,

    /// include messages marked as read too
    #[serde(default)]
    pub include_read: bool,
}

/// create a new message reminder notification
#[record]
pub struct NotificationCreate {
    /// the channel this message was sent in
    pub channel_id: ChannelId,

    /// the id of the message that was sent
    pub message_id: MessageId,

    /// set this in the future to create a reminder
    pub added_at: Option<Time>,
}

/// mark some notifications as read (or unread)
#[record]
pub struct NotificationMarkRead {
    /// mark these messages as read
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub message_ids: Vec<MessageId>,

    /// mark everything in these threads as read
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub channel_ids: Vec<ChannelId>,

    /// mark everything in these rooms as read
    #[serde(default)]
    #[schema(required = false, min_length = 1, max_length = 1024)]
    #[validate(length(min = 1, max = 1024))]
    pub room_ids: Vec<RoomId>,

    /// mark everything as read
    #[serde(default)]
    pub everything: bool,
}

/// delete some notifications
#[record]
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
    #[serde(default)]
    pub include_unread: bool,
}

/// paginate through your notifications
#[record]
pub struct NotificationPagination {
    pub notifications: Vec<Notification>,
    pub total: u64,
    pub cursor: Option<String>,

    #[deprecated = "check `cursor` field instead"]
    pub has_more: bool,

    // extra context
    pub messages: Vec<Message>,
    pub threads: Vec<Channel>,
    pub room_members: Vec<RoomMember>,
    pub thread_members: Vec<ThreadMember>,
    pub users: Vec<User>,
}

impl NotificationType {
    /// get the tag for this notification
    ///
    /// notifications with the same tag will be deduplicated
    // TODO: also deduplicate in inbox?
    // TODO: new NotificationTagId type?
    pub fn tag_id(&self) -> Uuid {
        match self {
            Self::Message { message_id, .. } => **message_id,
            Self::Reaction { message_id, .. } => **message_id,
            Self::Thread { thread_id, .. } => **thread_id,
            Self::FriendRequestSent { user_id } => **user_id,
            Self::FriendRequestReceived { user_id } => **user_id,
            Self::FriendRequestAccepted { user_id } => **user_id,
        }
    }

    pub fn message_id(&self) -> Option<MessageId> {
        match self {
            Self::Message { message_id, .. } => Some(*message_id),
            Self::Reaction { message_id, .. } => Some(*message_id),
            Self::Thread { .. } => None,
            Self::FriendRequestSent { .. } => None,
            Self::FriendRequestReceived { .. } => None,
            Self::FriendRequestAccepted { .. } => None,
        }
    }

    pub fn channel_id(&self) -> Option<ChannelId> {
        match self {
            Self::Message { channel_id, .. } => Some(*channel_id),
            Self::Reaction { channel_id, .. } => Some(*channel_id),
            Self::Thread { thread_id, .. } => Some(*thread_id),
            Self::FriendRequestSent { .. } => None,
            Self::FriendRequestReceived { .. } => None,
            Self::FriendRequestAccepted { .. } => None,
        }
    }

    pub fn room_id(&self) -> Option<RoomId> {
        match self {
            Self::Message { room_id, .. } => *room_id,
            Self::Reaction { room_id, .. } => *room_id,
            Self::Thread { room_id, .. } => *room_id,
            Self::FriendRequestSent { .. } => None,
            Self::FriendRequestReceived { .. } => None,
            Self::FriendRequestAccepted { .. } => None,
        }
    }
}

impl Notification {
    #[inline]
    pub fn tag_id(&self) -> Uuid {
        self.ty.tag_id()
    }

    #[inline]
    pub fn channel_id(&self) -> Option<ChannelId> {
        self.ty.channel_id()
    }

    #[inline]
    pub fn room_id(&self) -> Option<RoomId> {
        self.ty.room_id()
    }

    #[inline]
    pub fn message_id(&self) -> Option<MessageId> {
        self.ty.message_id()
    }
}
