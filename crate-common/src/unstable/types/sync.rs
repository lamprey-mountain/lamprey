#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::SessionToken;

// pub enum MessageClient {
//     /// initial message
//     Hello {
//         // TODO: include this (or something more structured)
//         user_agent: String,
//     },

//     /// replace current filter
//     Subscribe {
//         // only receive events from this room
//         room_id: Vec<RoomId>,

//         // TODO(#368): pubsub for single items?
//         invites: Vec<InviteCode>,
//         media: Vec<MediaId>,
//     },
// }

// maybe copy discord intents here
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
// i should probably add some way to shard; see twitch's conduit system
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
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
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "op"))]
pub enum Event<R, T> {
    /// heartbeat
    Ping,

    /// some kind of error
    Error { error: String },

    /// successfully connected
    Ready {
        /// ready data
        #[cfg_attr(feature = "serde", serde(flatten))]
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
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "op"))]
pub enum Command<R, T> {
    /// initial message
    Hello {
        /// authorization token
        token: SessionToken,

        /// extra data for hello
        #[cfg_attr(feature = "serde", serde(flatten))]
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
        #[cfg_attr(feature = "serde", serde(flatten))]
        data: T,
    },
}
