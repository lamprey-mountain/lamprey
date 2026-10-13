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

pub use builder::VoiceBuilder;
pub use client::{ConnectionState, Tracks, Voice};
pub use error::VoiceError;
pub use track::{Inbound, InboundStream, Outbound};

impl Client {
    /// create a voice connection
    pub fn voice(&self, channel_id: ChannelId) -> VoiceBuilder<'_> {
        VoiceBuilder::new(self, channel_id)
    }
}
