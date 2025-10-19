use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use std::collections::HashMap;

use crate::v1::types::{
    util::Time, Channel, ChannelId, Invite, InviteCode, Message, MessageId, MessageVerId,
    Relationship, Role, RoleId, Room, RoomId, RoomMember, Session, SessionId, ThreadMember, User,
    UserId,
};

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
        thread_id: ChannelId,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageSynchronizeRoom {
    pub room_id: RoomId,
    pub role: HashMap<RoleId, Option<Role>>,
    pub room_member: HashMap<(RoomId, UserId), Option<RoomMember>>,
    pub thread: HashMap<ChannelId, Option<Channel>>,
    // pub application: HashMap<UserId, Option<Application>>,
}

/// an upsert to a thread's state
#[derive(Debug, Clone, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageSynchronizeThread {
    pub thread_id: ChannelId,
    pub thread_member: HashMap<(ChannelId, UserId), Option<ThreadMember>>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// a payload for an experimental state-based sync method
// problems with state-based sync:
// - its less efficient; i can't patch
// - some events (like typing) don't really map to state naturally
// i'm likely not going to impl this, may remove later
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        thread_id: ChannelId,
        user_id: UserId,
        until: Time,
    },

    /// arbitrary user defined event, for bots? with a matching MessageClient
    /// entry? builtin pub/sub for bots?
    Dispatch {
        action: String,
        payload: Option<serde_json::Value>,
    },
}
