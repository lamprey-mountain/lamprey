use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{ChannelId, RoomId, SessionToken};

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

// // how do i add in extra stuff?
// struct AuditExtra {
//     // audit logs
//     by: Option<UserId>,
//     reason: Option<String>,
//
//     // ideompotency
//     nonce: Option<String>,
// }

// TODO(#871): reuse sync wrapper/transport
// use this for both client syncing and voice server syncing

/// an event from the server for the client
#[derive(Debug, Clone, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "op")]
pub enum Event<R, T> {
    /// heartbeat
    Ping,

    /// some kind of error
    Error { error: String },

    /// successfully connected
    Ready {
        /// ready data
        #[serde(flatten)]
        data: R,

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

    /// data to keep local copy of state in sync with server
    Dispatch { data: T, seq: u64 },
}

/// a command from the client to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "op")]
pub enum Command<R, T> {
    /// initial message
    Hello {
        /// authorization token
        token: SessionToken,

        /// extra data for hello
        #[serde(flatten)]
        data: R,
    },

    /// reconnect a dropped connection
    Resume {
        /// authorization token
        token: SessionToken,

        /// connection id
        conn: String,

        /// last seen sequence number
        seq: u64,
    },

    /// heartbeat
    Pong,

    /// send some data to the server
    Dispatch {
        #[serde(flatten)]
        data: T,
    },
}

/// errors you may receive
pub enum SyncError<T> {
    /// you were sent a Ping but didn't respond with a Pong in time
    Timeout,

    /// you tried to do something that you can't do
    Unauthorized,

    /// you tried to send a Hello or Resume but were already authenticated
    Unauthenticated,

    /// the token sent in Hello or Resume is invalid
    AuthFailure,

    /// you sent data that i couldn't decode
    InvalidData,

    /// you sent a sequence number that was invalid
    InvalidSequence,

    /// you're sending requests too quickly
    Ratelimit,

    /// sync specific error
    Custom(T),
}
