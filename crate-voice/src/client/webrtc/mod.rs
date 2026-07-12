use std::collections::VecDeque;

use common::v1::types::voice::messages::{PeerEvent, SignallingCommand, SignallingEvent};
use common::v1::types::voice::{KeyframeRequestKind, VoiceState};
use str0m::Rtc;

use crate::client::webrtc::mapping::Mapping;
use crate::prelude::*;
use crate::util::SfuVoiceState;
use crate::{
    client::webrtc::{datachannels::Datachannels, signalling::Signalling},
    util::permissions::Permissions,
};

pub mod datachannels;
pub mod mapping;
pub mod signalling;
pub mod track;

// PERF: maybe Box some stuff? unsure how big everything is.
/// webrtc connection state
pub struct Webrtc {
    vs: SfuVoiceState,
    rtc: Rtc,
    signalling: Signalling,
    datachannels: Datachannels,
    mapping: Mapping,
    events: VecDeque<PeerEvent>,
}

// TODO: figure out how big webrtc is
// #[test]
// fn arst() {
//     assert_eq!(std::mem::size_of::<Webrtc>(), 0);
// }

impl Webrtc {
    pub fn new(rtc: Rtc) -> Self {
        todo!()
    }

    // /// get a track id from this peer's local mid
    // pub fn lookup_track(&self, mid: SMid) -> Option<TrackId> {
    //     todo!()
    // }

    // pub fn write_media(&mut self, track_id: TrackId, media: &str0m::media::MediaData) {
    //     todo!()
    // }

    /// request a keyframe to be generated
    pub fn request_keyframe(
        &mut self,
        track: TrackSlot,
        rid: Option<SRid>,
        kind: KeyframeRequestKind,
    ) -> Result<()> {
        // Err(Error::TrackDoesntExist);
        todo!()
    }

    /// handle a signalling command
    pub fn handle_signalling(&mut self, cmd: SignallingCommand) {
        // self.events.push_back(PeerEvent::Signalling(SignallingEvent::Connected { sfu_id: () }));
        // self.events.drain(..);
        todo!()
    }

    /// get permissions for this peer
    ///
    /// resolves from voice state and room permissions
    pub fn permissions(&self) -> Permissions {
        todo!()
    }

    pub fn voice_state(&self) -> Option<&VoiceState> {
        todo!()
    }

    // TODO
    // pub fn mapping()
    // pub fn mapping_mut()

    pub fn poll_output(&mut self) -> Result<SOutput> {
        Ok(self.rtc.poll_output()?)
    }
}
