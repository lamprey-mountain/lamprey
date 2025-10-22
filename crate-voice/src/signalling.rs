use anyhow::{bail, Result};
use common::v1::types::voice::SessionDescription;
use str0m::{
    change::{SdpAnswer, SdpApi, SdpOffer, SdpPendingOffer},
    Rtc,
};
use tracing::{info, warn};

#[derive(Debug)]
pub enum SignallingState {
    Stable,
    HaveLocalOffer(SdpPendingOffer),
}

#[derive(Debug)]
pub struct Signalling {
    state: SignallingState,
}

impl Signalling {
    pub fn new() -> Self {
        Self {
            state: SignallingState::Stable,
        }
    }

    /// handle an sdp answer from the peer
    pub fn handle_answer(&mut self, rtc: &mut Rtc, sdp: SessionDescription) -> Result<()> {
        let pending = match std::mem::replace(&mut self.state, SignallingState::Stable) {
            SignallingState::HaveLocalOffer(pending) => pending,
            SignallingState::Stable => {
                warn!("received answer but we don't have a local offer");
                return Ok(());
            }
        };

        let answer = SdpAnswer::from_sdp_string(&sdp)?;
        rtc.sdp_api().accept_answer(pending, answer)?;
        info!("accept answer");

        Ok(())
    }

    /// handle an sdp offer from the peer
    pub fn handle_offer(&mut self, rtc: &mut Rtc, sdp: SessionDescription) -> Result<SdpAnswer> {
        if !matches!(self.state, SignallingState::Stable) {
            warn!("offer collision, but we are polite offer so allow it");
        }

        let offer = SdpOffer::from_sdp_string(&sdp)?;
        let answer = rtc.sdp_api().accept_offer(offer)?;
        self.state = SignallingState::Stable;

        Ok(answer)
    }

    /// send an sdp offer if we have tracks that haven't been negotiated yet
    pub fn negotiate_if_needed(&mut self, change: SdpApi) -> Result<Option<SdpOffer>> {
        if matches!(self.state, SignallingState::HaveLocalOffer(_)) {
            warn!("trying to negotiate, but we already have a local offer");
            return Ok(None);
        }

        if !change.has_changes() {
            return Ok(None);
        }

        let Some((offer, pending)) = change.apply() else {
            bail!("failed to apply sdp changes");
        };

        self.state = SignallingState::HaveLocalOffer(pending);

        Ok(Some(offer))
    }
}
