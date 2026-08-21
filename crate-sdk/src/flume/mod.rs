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
