//! a subscription to a channel
//!
//! allows receiving channel and message events

use crate::{
    v1::types::{ChannelId, ChannelSeq, flume::FlumeDeltaCanonical},
    v2::types::{MessageId, sync::stream::StreamProtocol},
};
use lamprey_macros::record;

pub struct Protocol;

#[record]
pub struct Initial {
    /// the id of the channel to subscribe to
    pub channel_id: ChannelId,

    /// the last sequence number the client has
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<ChannelSeq>,
}

#[record]
#[serde(tag = "op")]
pub enum Command {
    // no commands
}

#[record]
#[serde(tag = "type")]
pub enum Event {
    /// incremental update to a flume
    FlumeDelta {
        channel_id: ChannelId,
        message_id: MessageId,
        delta: FlumeDeltaCanonical,
    },

    /// initial snapshot of a flume
    ///
    /// when a user initially connects, this event is sent for each active flume
    FlumeInit {
        channel_id: ChannelId,
        message_id: MessageId,
        delta: FlumeDeltaCanonical,
    },

    /// confirmation that the client is now subscribed to the channel
    Subscribed,
}

impl StreamProtocol for Protocol {
    type Initial = Initial;
    type Command = Command;
    type Event = Event;
}
