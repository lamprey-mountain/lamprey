use common::v2::types::ChannelId;

use crate::Client;

mod builder;
mod client;
mod driver;
mod error;
mod track;

// TODO: impl this
// mod datachannel;
// mod player;

// TODO: store all voice states for all channels in client
// map of channel id -> voice state
// map of user id -> voice state

pub use builder::VoiceBuilder;
pub use client::{ConnectionState, Tracks, Voice};
pub use error::VoiceError;
pub use track::{Inbound, InboundStream, Outbound};

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
