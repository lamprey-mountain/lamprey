// NOTE: webhooks will probably only support dispaches

use lamprey_macros::record;

use crate::v2::types::sync::{
    dispatch::Dispatch,
    transport::{Compression, Encoding, Version},
};

#[record]
pub struct Config {
    pub version: Version,

    pub compression: Option<Compression>,

    #[serde(default)]
    pub encoding: Encoding,
}

/// an event from the server to a webhook
///
/// the webhook must respond with a 2xx status code within 3 seconds
#[record]
pub struct Envelope {
    pub version: Version,

    #[serde(flatten)]
    pub event: Event,
    // TODO: add more fields
    // pub connection_id: ConnectionId,
    // do i want to reuse ConnectionId to identify webhooks or create some webhook id type
    // should i include {session, application, user} ids too?
}

#[record]
#[serde(tag = "op")]
pub enum Event {
    /// heartbeat
    ///
    /// webhook should respond with 204 no content. make sure to validate that the signature is correct.
    Ping,

    /// a dispatch
    Dispatch {
        /// the sync dispatch itself
        // PERF: this should probably be batched as a Vec
        dispatch: Box<Dispatch>,
        // NOTE: do i need this?
        // /// the connection sequence number of this event, for resuming
        // seq: u64,
    },
}
