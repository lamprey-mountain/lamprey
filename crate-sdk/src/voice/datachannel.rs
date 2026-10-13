// THIS MAY BE REMOVED SOON

// use common::v1::types::voice::{
//     TrackId,
//     datachannel::{SpeakingDatagram, SpeakingFlags},
// };
// use futures_util::{StreamExt, stream::BoxStream};

// use crate::voice::VoiceError;

/// datachannels registry
///
/// stores all currently open datachannels
#[derive(Debug, Default)]
pub struct Datachannels {
    // pub(super) channels: DashMap<ChannelId, ()>,
    // TODO: implement
}

impl Datachannels {
    /// get or create the speaking indicator datachannel
    pub fn obtain_speaking(&self) -> () {
        todo!()
    }
}

// pub struct Speaking {
//     // TODO
// }

// impl Speaking {
//     /// send a speaking event
//     pub fn send(&self, _flags: SpeakingFlags, _track: TrackId) -> Result<(), VoiceError> {
//         todo!()
//     }

//     /// get a stream of speaking events
//     pub fn events(&self) -> BoxStream<'static, SpeakingDatagram> {
//         // TODO
//         futures_util::stream::empty().boxed()
//     }
// }
