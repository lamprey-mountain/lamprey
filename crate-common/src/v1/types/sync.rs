use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use crate::v1::types::{
    application::Connection, user_status::StatusPatch, util::Time, ApplicationId, AuditLogEntry,
    InviteTargetId, InviteWithMetadata, Relationship, RoomBan, ThreadMember,
};

use super::{
    emoji::EmojiCustom,
    reaction::ReactionKey,
    role::RoleReorderItem,
    user_config::{UserConfigGlobal, UserConfigRoom, UserConfigThread, UserConfigUser},
    voice::{SignallingMessage, VoiceState},
    EmojiId, InviteCode, Message, MessageId, MessageVerId, Role, RoleId, Room, RoomId, RoomMember,
    Session, SessionId, SessionToken, Thread, ThreadId, User, UserId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageClient {
    /// initial message
    Hello {
        token: SessionToken,

        status: Option<StatusPatch>,

        #[serde(flatten)]
        resume: Option<SyncResume>,
    },

    /// set status
    Status { status: StatusPatch },

    /// heartbeat
    Pong,

    /// send arbitrary data to a voice server
    // NOTE: should i split this into multiple messages? i'll probably keep it how it is currently tbh
    // TODO: handle multiple connections/servers (or find out how to split one connection amongst multiple hosts?)
    VoiceDispatch {
        user_id: UserId,
        payload: SignallingMessage,
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
#[allow(clippy::large_enum_variant)]
pub enum MessagePayload {
    /// heartbeat
    Ping,

    /// data to keep local copy of state in sync with server
    Sync { data: MessageSync, seq: u64 },

    /// some kind of error
    Error { error: String },

    /// successfully connected
    Ready {
        /// current user, null if session is unauthed
        user: Option<User>,

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
#[allow(clippy::large_enum_variant)]
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

    ThreadCreate {
        thread: Thread,
    },

    ThreadUpdate {
        thread: Thread,
    },

    ThreadTyping {
        thread_id: ThreadId,
        user_id: UserId,
        until: Time,
    },

    /// read receipt update
    ThreadAck {
        thread_id: ThreadId,
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
        /// deprecated = "keyed by thread_id"
        #[cfg_attr(feature = "utoipa", schema(deprecated))]
        room_id: Option<RoomId>,
        thread_id: ThreadId,
        message_id: MessageId,
    },

    MessageVersionDelete {
        /// deprecated = "keyed by thread_id"
        #[cfg_attr(feature = "utoipa", schema(deprecated))]
        room_id: Option<RoomId>,
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    },

    /// delete multiple messages at once
    MessageDeleteBulk {
        thread_id: ThreadId,
        message_ids: Vec<MessageId>,
    },

    MessageRemove {
        thread_id: ThreadId,
        message_ids: Vec<MessageId>,
    },

    MessageRestore {
        thread_id: ThreadId,
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
        invite: InviteWithMetadata,
    },

    InviteUpdate {
        invite: InviteWithMetadata,
    },

    InviteDelete {
        code: InviteCode,
        target: InviteTargetId,
    },

    ReactionCreate {
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    },

    ReactionDelete {
        user_id: UserId,
        thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    },

    /// remove all reactions
    ReactionPurge {
        thread_id: ThreadId,
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

    UserConfigGlobal {
        user_id: UserId,
        config: UserConfigGlobal,
    },

    UserConfigRoom {
        user_id: UserId,
        room_id: RoomId,
        config: UserConfigRoom,
    },

    UserConfigThread {
        user_id: UserId,
        thread_id: ThreadId,
        config: UserConfigThread,
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
        relationship: Relationship,
    },

    RelationshipDelete {
        user_id: UserId,
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
