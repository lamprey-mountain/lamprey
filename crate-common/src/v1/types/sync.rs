#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{
    application::Connection,
    automod::{AutomodRule, AutomodRuleExecution},
    presence::Presence,
    util::Time,
    voice::Call,
    webhook::Webhook,
    ApplicationId, AuditLogEntry, AutomodRuleId, CalendarEventId, InviteTargetId,
    InviteWithMetadata, Relationship, RoomBan, ThreadMember, WebhookId,
};

use crate::v2::types::message::Message as MessageV2;

use super::{
    calendar::{CalendarEvent, CalendarEventParticipant, CalendarOverwrite},
    emoji::EmojiCustom,
    notifications::{Notification, NotificationFlush, NotificationMarkRead},
    reaction::ReactionKey,
    role::RoleReorderItem,
    user_config::{UserConfigChannel, UserConfigGlobal, UserConfigRoom, UserConfigUser},
    voice::{SignallingMessage, VoiceState},
    Channel, ChannelId, EmojiId, InviteCode, MessageId, MessageVerId, Role, RoleId, Room, RoomId,
    RoomMember, Session, SessionId, SessionToken, User, UserId, Harvest,
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

// NOTE: consider making Ready and ReadySupplemental part of Sync
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "op")]
pub enum MessagePayload {
    /// heartbeat
    Ping,

    /// data to keep local copy of state in sync with server
    Sync {
        /// the data for this sync event
        data: Box<MessageSync>,

        /// the sequence number of this event, for resuming
        seq: u64,

        /// the nonce, if this is in response to a request with the `Idempotency-Key` header set
        #[serde(skip_serializing_if = "Option::is_none")]
        nonce: Option<String>,
    },

    /// some kind of error
    Error {
        error: String,
        // TODO(#918): code: SyncError,
    },

    /// successfully connected
    Ready {
        /// current user, null if session is unauthed
        user: Box<Option<User>>,

        // /// the application this bot user belongs, if the user is a bot
        // application: Box<Option<Application>>,
        /// current session
        session: Session,

        /// connection id
        conn: String,

        /// sequence id for reconnecting
        seq: u64,
    },

    /// send all missed messages, now tailing live event stream
    Resumed,

    /// client needs to disconnect and reconnect
    Reconnect {
        /// whether the client can resume
        can_resume: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageSync {
    // TODO: move Ready here
    // /// successfully connected
    // Ready {
    //     /// current user, null if session is unauthed
    //     user: Box<Option<User>>,

    //     // /// the application this bot user belongs, if the user is a bot
    //     // application: Box<Option<Application>>,
    //     /// current session
    //     session: Session,

    //     /// connection id
    //     conn: String,
    // },

    // /// extra data for the client to function, sent after Ready
    // ReadySupplemental {
    //     /// all rooms the user can see
    //     rooms: Vec<Room>,

    //     /// all roles in all rooms the user can see
    //     roles: Vec<Role>,

    //     /// all channels the user can see
    //     channels: Vec<Channel>,

    //     /// all threads the user can see
    //     threads: Vec<Channel>,

    //     /// only contains the auth user's members (one for each room)
    //     room_members: Vec<RoomMember>,

    //     /// user's global config
    //     config: UserConfigGlobal,

    //     // unsure about these
    //     friends: Vec<User>,
    //     emojis: Vec<CustomEmoji>,
    // },
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
        // i know, it's cursed to return v2 messages in a v1 api. but this is still in pre alpha so i don't really care.
        message: MessageV2,
        // /// the room member of the author, if this was sent in a room
        // room_member: Option<RoomMember>,

        // /// the thread member of the author, if this was sent in a thread
        // thread_member: Option<ThreadMember>,

        // /// the user who sent this message
        // user: User,
    },

    MessageUpdate {
        message: MessageV2,
        // /// the room member of the author, if this was sent in a room
        // room_member: Option<RoomMember>,

        // /// the thread member of the author, if this was sent in a thread
        // thread_member: Option<ThreadMember>,

        // /// the user who sent this message
        // user: User,
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

        // TODO: remove `message_ids`
        message_ids: Vec<MessageId>,
        // TODO: add `messages`
        // messages: Vec<Message>,
    },

    HarvestUpdate { harvest: Harvest },

    RoomMemberCreate {
        member: RoomMember,
        // user: User,
    },

    RoomMemberUpdate {
        member: RoomMember,
        // user: User,
    },

    RoomMemberDelete {
        room_id: RoomId,
        user_id: UserId,
    },

    // TODO: deprecate and remove
    RoomMemberUpsert {
        member: RoomMember,
    },

    // TODO: allow batch upserting/removing
    ThreadMemberUpsert {
        member: ThreadMember,
        // thread_id: ChannelId,
        // added: Vec<ThreadMember>,
        // removed: Vec<UserId>,
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

    /// remove one specific emoji on a message
    ReactionDelete {
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKey,
    },

    /// remove all reactions for a reaction key on a message
    ReactionDeleteKey {
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKey,
    },

    /// remove all reactions on a message
    ReactionDeleteAll {
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

    CallCreate {
        call: Call,
    },

    CallUpdate {
        call: Call,
    },

    CallDelete {
        channel_id: ChannelId,
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

    SessionDeleteAll {
        user_id: UserId,
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

    CalendarOverwriteCreate {
        channel_id: ChannelId,
        overwrite: CalendarOverwrite,
    },

    CalendarOverwriteUpdate {
        channel_id: ChannelId,
        overwrite: CalendarOverwrite,
    },

    CalendarOverwriteDelete {
        channel_id: ChannelId,
        event_id: CalendarEventId,
        seq: u64,
    },

    CalendarRsvpCreate {
        channel_id: ChannelId,
        event_id: CalendarEventId,
        participant: CalendarEventParticipant,
    },

    CalendarRsvpDelete {
        channel_id: ChannelId,
        event_id: CalendarEventId,
        user_id: UserId,
    },

    CalendarOverwriteRsvpCreate {
        channel_id: ChannelId,
        event_id: CalendarEventId,
        seq: u64,
        participant: CalendarEventParticipant,
    },

    CalendarOverwriteRsvpDelete {
        channel_id: ChannelId,
        event_id: CalendarEventId,
        seq: u64,
        user_id: UserId,
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

    // TODO: split out AutomodManage with RoomManage?
    /// an auto moderation rule was created. only sent to users with RoomManage.
    AutomodRuleCreate {
        rule: AutomodRule,
    },

    /// an auto moderation rule was updated. only sent to users with RoomManage.
    AutomodRuleUpdate {
        rule: AutomodRule,
    },

    /// an auto moderation rule was deleted. only sent to users with RoomManage.
    AutomodRuleDelete {
        rule_id: AutomodRuleId,
        room_id: RoomId,
    },

    /// an auto moderation rule was executed. only sent to users with RoomManage.
    AutomodRuleExecute {
        execution: AutomodRuleExecution,
    },

    RatelimitUpdate {
        channel_id: ChannelId,
        user_id: UserId,
        slowmode_thread_expire_at: Option<Time>,
        slowmode_message_expire_at: Option<Time>,
    },
    // TODO(#915): media v2
    // /// A piece of media has processed and is now in the `Uploaded` state.
    // MediaProcessed {
    //     media: crate::v2::types::media::Media,
    // },
}

// TODO: skip sending room_members/thread_members/users if the client already has them
// TODO: move member list stuff to a submodule
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
        // /// the users in this range
        // items: Vec<UserId>,
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MemberListGroupId {
    /// online members without a hoisted role
    Online,

    /// offline members, including those with a role
    Offline,

    /// hoisted roles
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncCompression {
    /// Deflate compression
    #[serde(rename = "deflate")]
    Deflate,
}
