use common::v1::types::voice::{
    TrackId,
    datachannel::{SpeakingDatagram, SpeakingFlags},
};
use futures_util::{StreamExt, stream::BoxStream};

use crate::voice::VoiceError;

pub struct Speaking {
    // TODO
}

impl Speaking {
    /// send a speaking event
    pub fn send(&self, _flags: SpeakingFlags, _track: TrackId) -> Result<(), VoiceError> {
        todo!()
    }

    /// get a stream of speaking events
    pub fn events(&self) -> BoxStream<'static, SpeakingDatagram> {
        // TODO
        futures_util::stream::empty().boxed()
    }
}
