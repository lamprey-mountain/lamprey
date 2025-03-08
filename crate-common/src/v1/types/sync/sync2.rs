use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

use std::collections::HashMap;

use crate::v1::types::{
    util::Time, Invite, InviteCode, Message, MessageId, MessageVerId, Relationship, Role, RoleId,
    Room, RoomId, RoomMember, Session, SessionId, Thread, ThreadId, ThreadMember, User, UserId,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
pub struct SyncParams {
    pub version: SyncVersion,
    pub compression: Option<SyncCompression>,
    pub format: SyncFormat,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncFilterId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncFilter {
    /// subscribe to **everything**
    SubAll {
        id: SyncFilterId,
    },

    /// subscribe to events in a room (excluding child thread events)
    SubRoom {
        id: SyncFilterId,
        room_id: RoomId,
    },

    /// subscribe to events in a room (including child thread events)
    SubRoomAll {
        id: SyncFilterId,
        room_id: RoomId,
    },

    /// subscribe to events in a thread
    SubThread {
        id: SyncFilterId,
        thread_id: ThreadId,
    },

    // SubEvents { id: SyncFilterId, want: SyncEventType },
    Unsub {
        id: SyncFilterId,
    },
}

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
// /// minimize bandwidth by only sending requested data
// pub enum SyncEventType {
//     Rooms,
//     RoomMembers,
//     Threads,
//     ThreadMembers,
//     Messages,
//     // maybe make this privileged a la discord?
//     // MessageContent,
//     Reactions,
//     UserStatus,
//     Typing,
//     Dms,
//     Voice,
// }

/// how to receive events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum SyncTransport {
    Webhook {
        url: String,
    },
    Websocket,
    /// long polling
    Poll,
}

// maybe use patches instead of upserts to save bandwidth?
// maybe not - patches only work if you already have a resource to patch
// None is a shorthand for state = Deleted
// maybe replace *Delete with *Upsert with state = deleted (but don't send actual full item content (hard to do while retaining compat?))

// FIXME: i don't know why utoipa is breaking here

/// an upsert to global state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageSynchronizeGlobal {
    pub room: HashMap<RoomId, Option<Room>>,
    pub user: HashMap<UserId, Option<User>>,
    pub dm: HashMap<UserId, Option<Room>>,
    pub relationship: HashMap<UserId, Option<Relationship>>,
    pub session: HashMap<SessionId, Option<Session>>,
    pub invite: HashMap<InviteCode, Option<Invite>>,
}

/// an upsert to a room's state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageSynchronizeRoom {
    pub room_id: RoomId,
    pub role: HashMap<RoleId, Option<Role>>,
    pub room_member: HashMap<(RoomId, UserId), Option<RoomMember>>,
    pub thread: HashMap<ThreadId, Option<Thread>>,
    // pub application: HashMap<UserId, Option<Application>>,
}

/// an upsert to a thread's state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageSynchronizeThread {
    pub thread_id: ThreadId,
    pub thread_member: HashMap<(ThreadId, UserId), Option<ThreadMember>>,
    // only one of message or message_version gets set per server event
    pub message: HashMap<MessageId, Option<Message>>,
    pub message_version: HashMap<MessageVerId, Option<Message>>,
    // pub voice: HashMap<UserId, Option<VoiceState>>,
    // pub reactions: HashMap<MessageId, Option<Reactions>>,
}

// // how do i add in extra stuff?
// struct AuditExtra {
//     // audit logs
//     by: Option<UserId>,
//     reason: Option<String>,
//
//     // ideompotency
//     nonce: Option<String>,
// }

/// a raw event from the server
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "op")]
pub enum Event {
    Ping,
    Error { error: String },
    Ready {},
    Resumed,
    Reconnect { can_resume: bool },
    Dispatch(Payload),
}

/// a payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "op")]
pub enum Payload {
    /// synchronize state with server
    SyncGlobal(MessageSynchronizeGlobal),

    /// synchronize room-specific state with server
    SyncRoom(MessageSynchronizeRoom),

    /// synchronize thread-specific state with server
    SyncThread(MessageSynchronizeThread),

    /// typing notification
    Typing {
        thread_id: ThreadId,
        user_id: UserId,
        until: Time,
    },

    /// arbitrary custom event
    Custom {
        name: String,
        payload: serde_json::Value,
    },
}
