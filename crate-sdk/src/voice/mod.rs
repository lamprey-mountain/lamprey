use common::v2::types::ChannelId;

use crate::Client;

mod builder;
mod client;
mod driver;
mod error;
mod player;
mod track;

// TODO: impl this
// mod datachannel;

pub use builder::VoiceBuilder;
pub use client::{ConnectionState, Tracks, Voice};
pub use error::VoiceError;

pub enum VoiceEvent {
    /// voice connection state changed
    StateChanged(ConnectionState),

    // /// a user is speaking
    // UserSpeaking(SpeakingWithUserId),
    /// the voice client has been disconnected
    Disconnected,

    /// an error occured
    Error(VoiceError),
}

impl Client {
    /// create a voice connection
    pub fn voice(&self, channel_id: ChannelId) -> VoiceBuilder<'_> {
        VoiceBuilder::new(self, channel_id)
    }
}
