use crate::prelude::*;
use str0m::{Rtc, channel::ChannelId};

pub struct Datachannels {
    speaking: Option<SChannelId>,
}

impl Datachannels {
    pub fn new() -> Self {
        Self { speaking: None }
    }

    /// get the datachannel for speaking/voice activity messages
    ///
    /// users send `Speaking` to sfus. sfus send `SpeakingWithUserId` to each other and users.
    pub fn speaking(&self) -> Option<SChannelId> {
        self.speaking
    }

    pub fn handle(&mut self, event: SEvent, rtc: &mut Rtc) {
        match event {
            SEvent::ChannelOpen(channel_id, _label) => {
                let chan = rtc.channel(channel_id).expect("guaranteed to exist");
                let config = chan
                    .config()
                    .expect("guaranteed to exist when ChannelOpen is emitted");
                match config.protocol.as_str() {
                    "speaking" => todo!(),
                    _ => {}
                }
            }
            SEvent::ChannelData(data) => {
                todo!()
            }
            SEvent::ChannelClose(channel_id) => {
                todo!()
            }
            // SEvent::ChannelBufferedAmountLow(channel_id) => todo!(),
            _ => return,
        }
    }
}
