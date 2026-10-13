//! Flumes are live updating messages.
//!
//! A flume is a message that's able to update its content in real time. Use the
//! [`Client::flume`] method to start building a flume via [`FlumeBuilder`], then
//! use [`FlumeWriter`] to send updates.
//!
//! [`FlumeReader`] is used to watch the flume's content.

use common::v2::types::ChannelId;

use crate::Client;

mod builder;
mod reader;
mod writer;

pub use builder::FlumeBuilder;
pub use reader::FlumeReader;
pub use writer::FlumeWriter;

// TODO: add an api to receive flumes
impl Client {
    /// create a new flume
    // NOTE: maybe this should take `create: FlumeCreate` and FlumeCreate should become a builder?
    pub fn flume(&self, channel_id: ChannelId) -> FlumeBuilder<'_> {
        FlumeBuilder::new(self, channel_id)
    }
}
