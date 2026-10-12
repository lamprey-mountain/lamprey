//! a subscription to a room
//!
//! allows receiving room and role events

use crate::{
    v1::types::{RoomId, mirror::RoomSeq, misc::Time},
    v2::types::{ChannelId, UserId, sync::stream::StreamProtocol},
};
use lamprey_macros::record;

pub struct Protocol;

#[record]
pub struct Initial {
    /// the id of the room to subscribe to
    pub room_id: RoomId,

    /// the last sequence number the client has
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<RoomSeq>,
}

#[record]
#[serde(tag = "op")]
pub enum Command {
    // no commands
}

#[record]
#[serde(tag = "op")]
pub enum Event {
    /// channel typing indicator
    Typing {
        channel_id: ChannelId,
        user_id: UserId,
        until: Time,
    },

    /// confirmation that the client is now subscribed to the room
    Subscribed,
}

impl StreamProtocol for Protocol {
    type Initial = Initial;
    type Command = Command;
    type Event = Event;
}
