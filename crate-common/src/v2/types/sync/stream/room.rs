//! a subscription to a room
//!
//! allows receiving room and role events

use crate::{
    v1::types::{RoomId, misc::Time},
    v2::types::{ChannelId, UserId, sync::stream::StreamProtocol},
};
use lamprey_macros::record;

pub struct Protocol;

#[record]
pub struct Initial {
    pub room_id: RoomId,
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
