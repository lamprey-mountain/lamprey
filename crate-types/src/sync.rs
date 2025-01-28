use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::{InviteTargetId, InviteWithMetadata};

use super::{
    InviteCode, Message, MessageId, MessageVerId, Role, RoleId, Room, RoomId, RoomMember, Session,
    SessionId, SessionToken, Thread, ThreadId, User, UserId,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageClient {
    /// initial message
    Hello {
        token: SessionToken,

        #[serde(flatten)]
        resume: Option<SyncResume>,
    },

    /// heartbeat
    Pong,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncResume {
    pub conn: String,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageEnvelope {
    #[serde(flatten)]
    pub payload: MessagePayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "op")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
pub enum MessageSync {
    UpsertRoom {
        room: Room,
    },
    UpsertThread {
        thread: Thread,
    },
    UpsertMessage {
        message: Message,
    },
    UpsertUser {
        user: User,
    },
    UpsertRoomMember {
        member: RoomMember,
    },
    UpsertSession {
        session: Session,
    },
    UpsertRole {
        role: Role,
    },
    UpsertInvite {
        invite: InviteWithMetadata,
    },
    DeleteMessage {
        thread_id: ThreadId,
        message_id: MessageId,
    },
    DeleteMessageVersion {
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    },
    DeleteUser {
        id: UserId,
    },
    DeleteSession {
        id: SessionId,
        user_id: Option<UserId>,
    },
    DeleteRole {
        room_id: RoomId,
        role_id: RoleId,
    },
    DeleteRoomMember {
        room_id: RoomId,
        user_id: UserId,
    },
    DeleteInvite {
        code: InviteCode,
        target: InviteTargetId,
    },
    Webhook {
        hook_id: Uuid,
        data: serde_json::Value,
    },
}

// /// messages specific to a user
// #[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
// #[serde(tag = "type")]
// enum MessageUser {}

// /// messages specific to a thread
// #[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
// #[serde(tag = "type")]
// enum MessageThread {}

// /// messages specific to a room
// #[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
// #[serde(tag = "type")]
// enum MessageRoom {}
