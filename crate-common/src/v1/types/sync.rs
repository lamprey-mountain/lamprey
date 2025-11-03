use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{
    application::Connection, presence::Presence, util::Time, webhook::Webhook, ApplicationId,
    AuditLogEntry, CalendarEventId, InviteTargetId, InviteWithMetadata, Relationship, RoomBan,
    ThreadMember, WebhookId,
};

use super::{
    calendar::CalendarEvent,
    emoji::EmojiCustom,
    notifications::{Notification, NotificationFlush, NotificationMarkRead},
    reaction::ReactionKey,
    role::RoleReorderItem,
    user_config::{UserConfigChannel, UserConfigGlobal, UserConfigRoom, UserConfigUser},
    voice::{SignallingMessage, VoiceState},
    Channel, ChannelId, EmojiId, InviteCode, Message, MessageId, MessageVerId, Role, RoleId, Room,
    RoomId, RoomMember, Session, SessionId, SessionToken, User, UserId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageClient {
    /// initial message
    Hello {
        token: SessionToken,

        presence: Option<Presence>,

        #[serde(flatten)]
        resume: Option<SyncResume>,
    },

    /// set presence
    Presence { presence: Presence },

    /// heartbeat
    Pong,

    /// send arbitrary data to a voice server
    // NOTE: should i split this into multiple messages? i'll probably keep it how it is currently tbh
    // TODO: handle multiple connections/servers (or find out how to split one connection amongst multiple hosts?)
    VoiceDispatch {
        user_id: UserId,
        payload: SignallingMessage,
    },

    /// subscribe to a range of room or thread members. you can subscribe to one list at a time.
    MemberListSubscribe {
        // TODO: rename thread_id -> channel_id
        // one of room_id or thread_id must be provided
        room_id: Option<RoomId>,
        thread_id: Option<ChannelId>,

        /// the ranges to subscribe to
        ranges: Vec<(u64, u64)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncResume {
    pub conn: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageEnvelope {
    #[serde(flatten)]
    pub payload: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "op")]
pub enum MessagePayload {
    /// heartbeat
    Ping,

    /// data to keep local copy of state in sync with server
    Sync { data: Box<MessageSync>, seq: u64 },

    /// some kind of error
    Error { error: String },

    /// successfully connected
    Ready {
        /// current user, null if session is unauthed
        user: Box<Option<User>>,

        /// current session
        session: Session,

        /// connection id
        conn: String,

        /// sequence id for reconnecting
        seq: u64,
    },

    /// successfully reconnected
    Resumed,

    /// client needs to disconnect and reconnect
    Reconnect { can_resume: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageSync {
    RoomCreate {
        room: Room,
    },

    RoomUpdate {
        room: Room,
    },

    RoomDelete {
        room_id: RoomId,
    },

    ChannelCreate {
        channel: Box<Channel>,
    },

    ChannelUpdate {
        channel: Box<Channel>,
    },

    ChannelTyping {
        channel_id: ChannelId,
        user_id: UserId,
        until: Time,
    },

    /// read receipt update
    ChannelAck {
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
    },

    MessageCreate {
        message: Message,
    },

    MessageUpdate {
        message: Message,
    },

    MessageDelete {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    MessageVersionDelete {
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
    },

    /// delete multiple messages at once
    MessageDeleteBulk {
        channel_id: ChannelId,
        message_ids: Vec<MessageId>,
    },

    MessageRemove {
        channel_id: ChannelId,
        message_ids: Vec<MessageId>,
    },

    MessageRestore {
        channel_id: ChannelId,
        // NOTE: if messages are not returned for listing endpoints, i should return a vec of Messages insetad
        message_ids: Vec<MessageId>,
    },

    RoomMemberUpsert {
        member: RoomMember,
    },

    ThreadMemberUpsert {
        member: ThreadMember,
    },

    RoleCreate {
        role: Role,
    },

    RoleUpdate {
        role: Role,
    },

    RoleDelete {
        room_id: RoomId,
        role_id: RoleId,
    },

    RoleReorder {
        room_id: RoomId,
        roles: Vec<RoleReorderItem>,
    },

    InviteCreate {
        invite: Box<InviteWithMetadata>,
    },

    InviteUpdate {
        invite: Box<InviteWithMetadata>,
    },

    InviteDelete {
        code: InviteCode,
        target: InviteTargetId,
    },

    ReactionCreate {
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKey,
    },

    ReactionDelete {
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKey,
    },

    /// remove all reactions
    ReactionPurge {
        channel_id: ChannelId,
        message_id: MessageId,
    },

    EmojiCreate {
        emoji: EmojiCustom,
    },

    EmojiUpdate {
        emoji: EmojiCustom,
    },

    EmojiDelete {
        emoji_id: EmojiId,
        room_id: RoomId,
    },

    /// receive a signalling message from a voice server
    VoiceDispatch {
        /// who to send this dispatch to
        user_id: UserId,
        payload: SignallingMessage,
    },

    VoiceState {
        user_id: UserId,
        state: Option<VoiceState>,

        // HACK: make it possible to use this for auth checks
        #[serde(skip)]
        old_state: Option<VoiceState>,
    },

    UserCreate {
        user: User,
    },

    UserUpdate {
        user: User,
    },

    PresenceUpdate {
        user_id: UserId,
        presence: Presence,
    },

    // TODO: rename these UserConfig -> Config
    UserConfigGlobal {
        user_id: UserId,
        config: UserConfigGlobal,
    },

    UserConfigRoom {
        user_id: UserId,
        room_id: RoomId,
        config: UserConfigRoom,
    },

    UserConfigChannel {
        user_id: UserId,
        channel_id: ChannelId,
        config: UserConfigChannel,
    },

    UserConfigUser {
        user_id: UserId,
        target_user_id: UserId,
        config: UserConfigUser,
    },

    UserDelete {
        id: UserId,
    },

    SessionCreate {
        session: Session,
    },

    SessionUpdate {
        session: Session,
    },

    SessionDelete {
        id: SessionId,
        user_id: Option<UserId>,
    },

    RelationshipUpsert {
        user_id: UserId,
        target_user_id: UserId,
        relationship: Relationship,
    },

    RelationshipDelete {
        user_id: UserId,
        target_user_id: UserId,
    },

    ConnectionCreate {
        user_id: UserId,
        connection: Connection,
    },

    ConnectionDelete {
        user_id: UserId,
        app_id: ApplicationId,
    },

    AuditLogEntryCreate {
        entry: AuditLogEntry,
    },

    BanCreate {
        room_id: RoomId,
        ban: RoomBan,
    },

    BanDelete {
        room_id: RoomId,
        user_id: UserId,
    },

    MemberListSync {
        /// which user this list sync is for
        user_id: UserId,
        room_id: Option<RoomId>,
        channel_id: Option<ChannelId>,
        ops: Vec<MemberListOp>,
        groups: Vec<MemberListGroup>,
    },

    InboxNotificationCreate {
        user_id: UserId,
        notification: Notification,
    },

    InboxMarkRead {
        user_id: UserId,
        #[serde(flatten)]
        params: NotificationMarkRead,
    },

    InboxMarkUnread {
        user_id: UserId,
        #[serde(flatten)]
        params: NotificationMarkRead,
    },

    InboxFlush {
        user_id: UserId,
        #[serde(flatten)]
        params: NotificationFlush,
    },

    CalendarEventCreate {
        event: CalendarEvent,
    },

    CalendarEventUpdate {
        event: CalendarEvent,
    },

    CalendarEventDelete {
        channel_id: ChannelId,
        event_id: CalendarEventId,
    },

    WebhookCreate {
        webhook: Webhook,
    },

    WebhookUpdate {
        webhook: Webhook,
    },

    WebhookDelete {
        webhook_id: WebhookId,
        room_id: Option<RoomId>,
        channel_id: ChannelId,
    },

    RatelimitUpdate {
        channel_id: ChannelId,
        user_id: UserId,
        slowmode_thread_expire_at: Option<Time>,
        slowmode_message_expire_at: Option<Time>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MemberListOp {
    /// replace a range of members
    Sync {
        /// the start of the range
        position: u64,

        /// only returned if channel is in a room
        room_members: Option<Vec<RoomMember>>,

        /// only returned if listing members in a thread
        thread_members: Option<Vec<ThreadMember>>,

        users: Vec<User>,
    },

    /// insert a member
    Insert {
        position: u64,
        room_member: Option<RoomMember>,
        thread_member: Option<ThreadMember>,
        user: Box<User>,
    },

    /// delete a range of one or more members
    Delete {
        position: u64,
        // usually will be 1
        count: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MemberListGroup {
    pub id: MemberListGroupId,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MemberListGroupId {
    /// online members without a hoisted role
    Online,

    /// offline members, including those with a role
    Offline,

    /// hoisted roles
    // TODO: implement role hoisting
    #[serde(untagged)]
    Role(RoleId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct SyncParams {
    pub version: SyncVersion,
    pub compression: Option<SyncCompression>,
    #[serde(default)]
    pub format: SyncFormat,
}

// i thought that putting the api version in the path would be better, but
// apparently websockets are hard to load balance. being able to use arbitrary
// urls/paths in the future could be helpful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[repr(u8)]
pub enum SyncVersion {
    V1 = 1,
}

impl Serialize for SyncVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for SyncVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match u8::deserialize(deserializer)? {
            1 => Ok(SyncVersion::V1),
            n => Err(serde::de::Error::unknown_variant(&n.to_string(), &["1"])),
        }
    }
}

// TODO(#249): websocket msgpack
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncFormat {
    #[default]
    Json,
    // Msgpack,
}

// TODO(#209): implement websocket compression
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncCompression {
    // Zlib, // new DecompressionStream("deflate")
}
