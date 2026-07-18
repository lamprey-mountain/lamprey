use common::v1::types::voice::internal::SfuVoiceState;
use common::v1::types::voice::{IceCandidate, MediaKind, SessionDescription, VoiceStateUpdate};
use common::v2::types::UserId;
use str0m::Rtc;
use tracing::debug;

use crate::client::webrtc::mapping::Mapping;
use crate::prelude::*;
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

    pub fn handle_event(&mut self, event: &SEvent) {
        self.datachannels.handle(event, &mut self.rtc);
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

    pub fn datachannels(&self) -> &Datachannels {
        &self.datachannels
    }

    pub fn rtc(&self) -> &Rtc {
        &self.rtc
    }

    pub fn rtc_mut(&mut self) -> &mut Rtc {
        &mut self.rtc
    }

    pub fn update_voice_state(&mut self, vs: VoiceStateUpdate) {
        self.vs.apply_update(vs);
    }

    pub fn disconnect(&mut self) {
        self.rtc.disconnect();
    }

    pub fn is_alive(&self) -> bool {
        self.rtc.is_alive()
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
        // NOTE: do not add ice candidates until remote description is fully set
        // if let Ok(c) = str0m::Candidate::from_sdp_string(&candidate) {
        //     self.rtc.add_remote_candidate(c);
        // }
    }

    /// get permissions for this peer
    ///
    /// resolves from voice state and room permissions
    pub fn permissions(&self) -> Permissions {
        Permissions::from_state(&self.vs)
    }

    pub fn voice_state(&self) -> &SfuVoiceState {
        &self.vs
    }

    pub fn user_id(&self) -> UserId {
        self.vs.user_id
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

    /// apply changes to outbound tracks, renegotiating if needed
    pub fn apply_outbound_changes(
        &mut self,
        pending: &[PeerChange],
    ) -> Result<Option<(Vec<(TrackSlot, SMid)>, SessionDescription)>> {
        let mut changes = self.rtc.sdp_api();
        let mut tracks = vec![];
        let mut new_mappings = vec![];
        for change in pending {
            match change {
                PeerChange::Open(slot, kind) => {
                    let mid =
                        changes.add_media((*kind).into(), SDirection::SendOnly, None, None, None);
                    tracks.push((*slot, mid));
                    // TODO: in ShardCall self.outbound[slot].state = TrackState::Negotiating(mid);
                    new_mappings.push((mid, *slot));
                }
                PeerChange::Close(mid) => changes.stop_media(*mid),
            }
        }

        // NOTE: neither of these two are needed?
        // changes.add_channel_with_config(str0m::channel::ChannelConfig { label: (), ordered: (), reliability: (), negotiated: (), protocol: () });
        // changes.set_direction(mid, dir);

        let out = match self.signalling.negotiate_if_needed(changes)? {
            Some(o) => {
                for (mid, slot) in new_mappings {
                    self.mapping.insert(mid, slot);
                }
                let sdp = SessionDescription(o.to_sdp_string());
                Some((tracks, sdp))
            }
            None => None,
        };
        Ok(out)
    }
}

pub enum PeerChange {
    Open(TrackSlot, MediaKind),
    Close(SMid),
}
