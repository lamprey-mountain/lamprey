//! a connection to a voice channel

use lamprey_macros::record;

use crate::{
    v1::types::voice::{
        VoiceStateUpdate,
        messages::{SignallingCommand, SignallingEvent},
    },
    v2::types::sync::stream::StreamProtocol,
};

pub struct Protocol;

#[record]
#[serde(transparent)]
pub struct Initial(pub VoiceStateUpdate);

#[record]
#[serde(transparent)]
pub struct Command(pub SignallingCommand);

#[record]
#[serde(transparent)]
pub struct Event(pub SignallingEvent);

impl StreamProtocol for Protocol {
    type Initial = Initial;
    type Command = Command;
    type Event = Event;
}
