use crate::prelude::*;
use str0m::Rtc;

/// datachannel registry
#[derive(Debug, Default)]
pub struct Datachannels {
    speaking: Option<SChannelId>,
}

impl Datachannels {
    /// create a new, empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// get the datachannel for speaking/voice activity messages
    ///
    /// users send `Speaking` to sfus. sfus send `SpeakingWithUserId` to each other and users.
    pub fn speaking(&self) -> Option<SChannelId> {
        self.speaking
    }

    pub fn handle(&mut self, event: &SEvent, rtc: &mut Rtc) {
        match event {
            SEvent::ChannelOpen(channel_id, _label) => {
                let chan = rtc.channel(*channel_id).expect("guaranteed to exist");
                let config = chan
                    .config()
                    .expect("guaranteed to exist when ChannelOpen is emitted");
                match config.protocol.as_str() {
                    "speaking" => self.speaking = Some(*channel_id),
                    _ => {}
                }
            }
            SEvent::ChannelClose(channel_id) => {
                if self.speaking == Some(*channel_id) {
                    self.speaking = None;
                }
            }
            _ => return,
        }
    }
}
