use std::collections::HashMap;

use common::v1::types::voice::messages::{SignallingCommand, SignallingEvent};
use common::v1::types::voice::SessionDescription;
use common::v1::types::voice::VoiceState;
use common::v2::types::UserId;
use str0m::Rtc;
use tracing::{debug, warn};

use crate::prelude::*;
use crate::signalling::Signalling;
use crate::util::TrackId;

/// a webrtc peer managed by an SfuShard
pub struct Webrtc {
    // TODO: remove, use voice_state.user_id
    pub user_id: UserId,

    /// webrtc logic via str0m
    pub rtc: Rtc,

    /// the user's voice state
    pub voice_state: VoiceState,

    pub signalling: Signalling,

    /// tracks coming from the peer to this sfu
    pub inbound: HashMap<SMid, TrackId>,

    /// tracks going from this sfu to the peer
    pub outbound: HashMap<SMid, TrackId>,

    /// tracks going from this sfu to the peer, indexed by TrackId
    pub outbound_mid: HashMap<TrackId, SMid>,

    /// tracks that need to be added to the peer connection
    pub pending_tracks: Vec<TrackId>,

    pub speaking_chan: Option<str0m::channel::ChannelId>,
}

impl Webrtc {
    pub fn new(user_id: UserId, voice_state: VoiceState, rtc: Rtc) -> Self {
        Self {
            user_id,
            rtc,
            voice_state,
            signalling: Signalling::new(),
            inbound: HashMap::new(),
            outbound: HashMap::new(),
            outbound_mid: HashMap::new(),
            pending_tracks: Vec::new(),
            speaking_chan: None,
        }
    }

    pub fn handle_signalling(&mut self, cmd: SignallingCommand) -> Option<SignallingEvent> {
        match cmd {
            SignallingCommand::Offer { sdp, tracks: _ } => {
                match self.signalling.handle_offer(&mut self.rtc, sdp) {
                    Ok(answer) => {
                        // Inbound tracks are handled by SfuShard, but we keep track of mid mapping
                        return Some(SignallingEvent::Answer {
                            sdp: SessionDescription(answer.to_sdp_string()),
                        });
                    }
                    Err(e) => {
                        warn!("Failed to handle offer: {:?}", e);
                    }
                }
            }
            SignallingCommand::Answer { sdp } => {
                if let Err(e) = self.signalling.handle_answer(&mut self.rtc, sdp) {
                    warn!("Failed to handle answer: {:?}", e);
                }
            }
            SignallingCommand::Candidate { candidate: _ } => {
                // TODO: ice candidates
                // if let Ok(c) = Candidate::from_sdp_string(&candidate.0) {
                //     self.rtc.add_remote_candidate(c);
                // }
            }
            SignallingCommand::Disconnect => {
                self.rtc.disconnect();
            }
            SignallingCommand::VoiceState { state } => {
                self.voice_state.apply(state);
            }
            SignallingCommand::Want { subscriptions } => {
                debug!("Want subscriptions: {:?}", subscriptions);
            }
        }
        None
    }

    pub fn negotiate_if_needed(&mut self) -> Option<SignallingEvent> {
        let change = self.rtc.sdp_api();
        if let Ok(Some(offer)) = self.signalling.negotiate_if_needed(change) {
            Some(SignallingEvent::Offer {
                sdp: SessionDescription(offer.to_sdp_string()),

                // populated by SfuShard
                // TODO: maybe use a different type than SignallingEvent since this isnt being used?
                tracks: vec![],
            })
        } else {
            None
        }
    }
}
