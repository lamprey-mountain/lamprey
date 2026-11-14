use crate::prelude::*;
use common::v1::types::voice::SessionDescription;
use str0m::{
    Rtc,
    change::{SdpAnswer, SdpApi, SdpOffer, SdpPendingOffer},
};
use tracing::{info, warn};

/// utility to handle webrtc signalling
#[derive(Debug)]
pub struct Signalling {
    state: SignallingState,
}

#[derive(Debug)]
pub enum SignallingState {
    Stable,
    HaveLocalOffer(LocalOffer),
}

/// a local offer that is waiting for the peer's answer
#[derive(Debug)]
pub struct LocalOffer {
    pending: SdpPendingOffer,
    /// mids added by this offer, needed to roll the tracks back if the offer never completes
    mids: Vec<SMid>,
}

/// the result of accepting an offer from the peer
#[derive(Debug)]
pub struct OfferOutcome {
    /// the answer to send back to the peer
    pub answer: SessionDescription,

    /// mids of a local offer that this offer rolled back
    ///
    /// the tracks behind these mids were never negotiated and have to be offered again
    pub rolled_back: Vec<SMid>,
}

/// the result of handling an answer from the peer
#[derive(Debug)]
pub enum AnswerOutcome {
    /// the answer completed the pending local offer
    Accepted,

    /// there was no pending local offer, so the answer was ignored
    Ignored,
}

impl Signalling {
    pub fn new() -> Self {
        Self {
            state: SignallingState::Stable,
        }
    }

    /// handle an sdp answer from the peer
    pub fn handle_answer(
        &mut self,
        rtc: &mut Rtc,
        sdp: SessionDescription,
    ) -> Result<AnswerOutcome> {
        let answer = SdpAnswer::from_sdp_string(&sdp)?;

        let local = match std::mem::replace(&mut self.state, SignallingState::Stable) {
            SignallingState::HaveLocalOffer(local) => local,
            SignallingState::Stable => {
                warn!("received answer but we don't have a local offer");
                return Ok(AnswerOutcome::Ignored);
            }
        };

        if let Err(e) = rtc.sdp_api().accept_answer(local.pending, answer) {
            // the session can't complete the offer this answer was meant for, and the
            // offer is gone now, so the peer has to reconnect
            warn!("failed to accept answer, disconnecting: {e}");
            rtc.disconnect();
            return Err(e.into());
        }

        info!("accept answer");

        Ok(AnswerOutcome::Accepted)
    }

    /// handle an sdp offer from the peer
    ///
    /// an offer from the peer always wins over a local offer that is still waiting for an
    /// answer: that offer is rolled back and its mids are returned in [`OfferOutcome`]
    pub fn handle_offer(&mut self, rtc: &mut Rtc, sdp: SessionDescription) -> Result<OfferOutcome> {
        let offer = SdpOffer::from_sdp_string(&sdp)?;

        // accepting an offer invalidates any pending local offer, so this is the rollback
        let rolled_back = match std::mem::replace(&mut self.state, SignallingState::Stable) {
            SignallingState::HaveLocalOffer(local) => {
                warn!("offer collision, rolling back local offer");
                local.mids
            }
            SignallingState::Stable => Vec::new(),
        };

        let answer = match rtc.sdp_api().accept_offer(offer) {
            Ok(answer) => answer,
            Err(e) => {
                // the session can't be recovered from an unacceptable offer, and the local
                // offer was already invalidated, so the peer has to reconnect
                rtc.disconnect();
                return Err(e.into());
            }
        };

        Ok(OfferOutcome {
            answer: SessionDescription(answer.to_sdp_string()),
            rolled_back,
        })
    }

    /// send an sdp offer if we have tracks that haven't been negotiated yet
    ///
    /// `mids` are the mids added by `change`. they are kept until the answer arrives so that a
    /// colliding offer from the peer can roll the tracks back
    pub fn negotiate_if_needed(
        &mut self,
        change: SdpApi,
        mids: Vec<SMid>,
    ) -> Result<Option<SdpOffer>> {
        if matches!(self.state, SignallingState::HaveLocalOffer(_)) {
            warn!("trying to negotiate, but we already have a local offer");
            return Ok(None);
        }

        if !change.has_changes() {
            return Ok(None);
        }

        let Some((offer, pending)) = change.apply() else {
            return Err(Error::Backend("failed to apply sdp changes".to_string()));
        };

        self.state = SignallingState::HaveLocalOffer(LocalOffer { pending, mids });

        Ok(Some(offer))
    }
}
