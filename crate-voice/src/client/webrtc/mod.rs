use std::collections::VecDeque;

use common::v1::types::voice::messages::{PeerEvent, SignallingCommand, SignallingEvent};
use common::v1::types::voice::{
    IceCandidate, KeyframeRequestKind, SessionDescription, VoiceState, VoiceStateUpdate,
};
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
        }
    }

    pub fn write_media(&mut self, track: TrackSlot, media: &str0m::media::MediaData) {
        let Some(mid) = self.mapping.lookup_mid(track) else {
            return;
        };

        let Some(writer) = self.rtc.writer(mid) else {
            return;
        };

        if let Some(pt) = writer.match_params(media.params) {
            let _ = writer.write(pt, media.network_time, media.time, Arc::clone(&media.data));
        }
    }

    /// request a keyframe to be generated
    pub fn request_keyframe(
        &mut self,
        track: TrackSlot,
        rid: Option<SRid>,
        kind: SKeyframeRequestKind,
    ) -> Result<()> {
        let Some(mid) = self.mapping.lookup_mid(track) else {
            // NOTE: maybe return Err(Error::TrackDoesntExist)
            return Ok(());
        };

        if let Some(mut w) = self.rtc.writer(mid) {
            let _ = w.request_keyframe(rid.map(Into::into), kind.into());
        }

        debug!("Keyframe requested for track {:?}", track);
        Ok(())
    }

    pub fn update_voice_state(&mut self, vs: VoiceStateUpdate) {
        self.vs.inner.apply(vs);
    }

    pub fn disconnect(&mut self) {
        self.rtc.disconnect();
    }

    pub fn handle_offer(&mut self, sdp: SessionDescription) -> Result<SessionDescription> {
        let answer = self.signalling.handle_offer(&mut self.rtc, sdp)?;
        Ok(SessionDescription(answer.to_sdp_string()))
    }

    pub fn handle_answer(&mut self, sdp: SessionDescription) {
        if let Err(e) = self.signalling.handle_answer(&mut self.rtc, sdp) {
            debug!("Failed to handle answer: {:?}", e);
        }
    }

    pub fn handle_candidate(&mut self, candidate: IceCandidate) {
        debug!("ignoring candidate: {:?}", candidate);
    }

    /// get a mutable sdp api handle to stage media changes
    pub fn sdp_api(&mut self) -> str0m::change::SdpApi<'_> {
        self.rtc.sdp_api()
    }

    /// send an sdp offer if we have tracks that haven't been negotiated yet
    pub fn negotiate_if_needed(
        &mut self,
        change: str0m::change::SdpApi,
    ) -> Result<Option<SessionDescription>> {
        if let Some(offer) = self.signalling.negotiate_if_needed(change)? {
            Ok(Some(SessionDescription(offer.to_sdp_string())))
        } else {
            Ok(None)
        }
    }

    // NOTE: this may be required for mutability reasons
    // /// Apply sdp changes via a closure, then send an offer if anything changed.
    // ///
    // /// The closure receives a mutable `SdpApi` and should call `add_media`/`stop_media` on it.
    // /// Combines the `sdp_api()` + `negotiate_if_needed()` pair into one call so that
    // /// `self.rtc` and `self.signalling` can be split-borrowed within this single method body.
    // pub fn negotiate_changes<F>(&mut self, f: F) -> Result<Option<SessionDescription>>
    // where
    //     F: FnOnce(&mut str0m::change::SdpApi),
    // {
    //     let mut changes = self.rtc.sdp_api();
    //     f(&mut changes);
    //     if let Some(offer) = self.signalling.negotiate_if_needed(changes)? {
    //         Ok(Some(SessionDescription(offer.to_sdp_string())))
    //     } else {
    //         Ok(None)
    //     }
    // }

    /// get permissions for this peer
    ///
    /// resolves from voice state and room permissions
    pub fn permissions(&self) -> Permissions {
        self.vs.permissions()
    }

    pub fn voice_state(&self) -> Option<&VoiceState> {
        Some(&self.vs.inner)
    }

    pub fn accepts(&self, input: &SInput) -> bool {
        self.rtc.accepts(input)
    }

    pub fn handle_input(&mut self, input: SInput) -> Result<()> {
        self.rtc.handle_input(input).map_err(Into::into)
    }

    pub fn mapping(&self) -> &Mapping {
        &self.mapping
    }

    pub fn mapping_mut(&mut self) -> &mut Mapping {
        &mut self.mapping
    }

    pub fn poll_output(&mut self) -> Result<SOutput> {
        Ok(self.rtc.poll_output()?)
    }
}
