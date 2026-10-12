//! protocol for the initial stream
//!
//! this stream must be created before anything else and is used for authentication

use lamprey_macros::record;

use crate::{
    v1::types::{SessionToken, presence::Presence},
    v2::types::{
        ConnectionId,
        sync::{
            dispatch::{Ready, global::DispatchGlobal},
            stream::StreamProtocol,
        },
    },
};

pub struct Protocol;

#[record]
#[serde(tag = "op")]
pub enum Initial {
    Identify(Identify),
    Resume(Resume),
    Shard(Shard),
}

#[record]
#[serde(tag = "op")]
pub enum Command {
    /// set presence for the current user
    Presence { presence: Presence },

    /// cleanly close this connection
    ///
    /// closes all streams and the connection
    // NOTE: how does this work for sharding?
    Close,

    /// heartbeat
    // NOTE: do i want to reverse heartbeats to be able to detect backpressure?
    Pong,
}

/// start a new sync connection
#[record]
pub struct Identify {
    /// authentication token
    pub token: SessionToken,

    /// initial presence to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<Presence>,

    pub properties: Properties,
}

/// resume an existing sync connection
#[record]
pub struct Resume {
    /// authentication token
    pub token: SessionToken,

    /// the id of the connection you're resuming
    pub connection_id: ConnectionId,

    /// the sequence number of the last sent event
    // TODO: remove?
    #[serde(default)]
    pub seq: u64,
    // TODO(?): do i include this? if not, remove.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub shard_id: Option<ShardId>,
}

/// shard the logical sync connection across multiple physical transports
#[record]
pub struct Shard {
    /// authentication token
    pub token: SessionToken,

    /// the id of the connection you're sharding
    pub connection_id: ConnectionId,
    // TODO(?): do i include this? if not, remove.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub shard_id: Option<ShardId>,
}

/// metadata about a connection
///
/// for use by the service operator when debugging or analyzing traffic
#[record]
pub struct Properties {
    /// info about the platform used to connect (eg. os, browser, device)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,

    /// info about the library/sdk used to connect
    pub library: Option<String>,

    /// info about the client/application that's connecting
    pub application: Option<String>,
}

#[record]
#[serde(tag = "op")]
pub enum Event {
    /// heartbeat
    Ping,

    /// all missed messages have been sent
    ///
    /// you are now tailing the live event stream
    Resumed,

    Ready(Ready),
    Dispatch(Dispatch),
    Goodbye(Goodbye),
}

/// data to keep local copy of state in sync with server
#[record]
pub struct Dispatch {
    /// the dispatch data itself
    pub inner: Box<DispatchGlobal>,

    /// the connection sequence number of this event, for resuming
    pub seq: u64,
    // /// the nonce for responses
    // ///
    // /// set if:
    // ///
    // /// - this is in response to a http request with the `Idempotency-Key` header set
    // /// - this is in response to a `SyncCommand` with an associated nonce
    // #[serde(skip_serializing_if = "Option::is_none")]
    // nonce: Option<String>,
}

/// server is disconnecting the client
#[record]
pub struct Goodbye {
    /// whether the client can resume this connection
    ///
    /// if false, the client may still be able to reconnect with their token (eg. resume seq is too old)
    pub resumable: bool,
    // // TODO: figure out how to make this work across all transports
    // /// the url that the client should reconnect to
    // ///
    // /// if unset, the client cannot reconnect (eg. the provided token is invalid)
    // reconnect_url: Option<Url>,
}

impl StreamProtocol for Protocol {
    type Initial = Initial;
    type Command = Command;
    type Event = Event;
}

// TODO: export models without causing name collisions
// export_models!(Initial, Command, Event)
