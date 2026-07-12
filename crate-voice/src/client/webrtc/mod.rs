use std::collections::VecDeque;

use common::v1::types::voice::messages::{PeerEvent, SignallingCommand, SignallingEvent};
use common::v1::types::voice::{KeyframeRequestKind, VoiceState};
use str0m::Rtc;
use tracing::debug;

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
    pub fn new(rtc: Rtc, vs: SfuVoiceState) -> Self {
        Self {
            vs,
            rtc,
            signalling: Signalling::new(),
            datachannels: Datachannels::new(),
            mapping: Mapping::new(),
            events: VecDeque::new(),
        }
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
        _track: TrackSlot,
        _rid: Option<SRid>,
        _kind: KeyframeRequestKind,
    ) -> Result<()> {
        // Err(Error::TrackDoesntExist);
        debug!("Keyframe requested");
        Ok(())
    }

    /// handle a signalling command
    pub fn handle_signalling(&mut self, _cmd: SignallingCommand) {
        // self.events.push_back(PeerEvent::Signalling(SignallingEvent::Connected { sfu_id: () }));
        // self.events.drain(..);
        debug!("Handling signalling command");
    }

    /// get permissions for this peer
    ///
    /// resolves from voice state and room permissions
    pub fn permissions(&self) -> Permissions {
        let p = &self.vs.permissions;
        let vs = &self.vs.inner;

        // TODO: impl ServerVoiceState { fn permissions(&self) -> Permissions {...}}

        Permissions {
            video: p.video(),
            audio: p.speak() && !vs.muted(),
            deaf: vs.deafened(),
        }
    }

    pub fn voice_state(&self) -> Option<&VoiceState> {
        Some(&self.vs.inner)
    }

    pub fn accepts(&self, input: &SInput) -> bool {
        self.rtc.accepts(input)
    }

    // TODO
    // pub fn mapping()
    // pub fn mapping_mut()

    pub fn poll_output(&mut self) -> Result<SOutput> {
        Ok(self.rtc.poll_output()?)
    }
}
