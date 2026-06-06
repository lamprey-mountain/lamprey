use common::v1::types::voice::{Mid, SpeakingFlags, SpeakingWithUserId};
use futures_util::{stream::BoxStream, StreamExt};

use crate::voice::VoiceError;

pub struct Speaking {
    //
}

impl Speaking {
    /// send a speaking event
    pub fn send(&self, _flags: SpeakingFlags, _mid: Mid) -> Result<(), VoiceError> {
        todo!()
    }

    /// get a stream of speaking events
    pub fn events(&self) -> BoxStream<'static, SpeakingWithUserId> {
        futures_util::stream::empty().boxed()
    }
}
