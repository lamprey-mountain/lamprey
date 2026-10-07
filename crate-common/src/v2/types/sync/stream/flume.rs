//! flume control stream
//!
//! each flume stream controls a flume

use crate::{
    v1::types::{ChannelId, Message, MessageId, message::flume::FlumeDeltaCreate},
    v2::types::sync::stream::StreamProtocol,
};
use lamprey_macros::record;

pub struct Protocol;

// client creates a flume via the flume_create endpoint (maybe merge with message_create?), then connects via a stream
// POST /channel/{channel_id}/flume -- do i keep any other endpoints?
// TODO: deprecate the http flume api

#[record]
pub struct Initial {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
}

// NOTE: alternative Initial message (probably should use an enum to require either message_id or create)
// #[record]
// pub struct Initial {
//     pub channel_id: ChannelId,
//     pub message_id: Option<MessageId>,
//
//     /// if no message_id is provided, create a new flume
//     pub create: Option<FlumeCreate>,
// }

#[record]
#[serde(tag = "op")]
pub enum Command {
    /// Keep the flume alive
    Ping,

    /// Commit the flume content
    Commit,

    /// Apply a patch to the flume's components
    Delta {
        delta: FlumeDeltaCreate,

        /// monotonic sequence number of this delta, used for acks
        seq: u64,
    },
}

#[record]
#[serde(tag = "op")]
pub enum Event {
    /// Flume committed successfully
    Committed { message: Box<Message> },

    /// Delta applied successfully
    Ack {
        /// the last successfully applied delta
        ///
        /// the server may throttle `Ack`s and skip sequence numbers instead of acking every `Delta`.
        seq: u64,
    },
}

// NOTE: i probably want to avoid echoing FlumeDelta dispatches back to the client through a channel subscription stream?
// NOTE: should i add ratelimits for flume deltas? how would i implement them?
// TODO: add some way to reconnect to a flume in case the stream or connection dies.

impl StreamProtocol for Protocol {
    type Initial = Initial;
    type Command = Command;
    type Event = Event;
}
