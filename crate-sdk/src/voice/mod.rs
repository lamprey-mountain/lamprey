//! The main entrypoint to the voice system is via [`Client::voice`], which returns a [`Voice`].

use common::v2::types::ChannelId;

use crate::Client;

mod builder;
mod client;
mod driver;
mod error;
mod player;
mod track;

// NOTE: i've tried to sketch some types in this module, but im not sure if they're any good.
// mod player_old;

// TODO: impl this
// mod datachannel;

pub use builder::VoiceBuilder;
pub use client::{ConnectionState, Tracks, Voice};
pub use error::VoiceError;
pub use player::{
    FileSource, Frame, Player, PlayerProducer, Producer, PublishBuilder, Source, Switchboard,
};
pub use track::{Inbound, InboundStream, Outbound};

impl Client {
    /// create a voice connection
    pub fn voice(&self, channel_id: ChannelId) -> VoiceBuilder<'_> {
        VoiceBuilder::new(self, channel_id)
    }
}
