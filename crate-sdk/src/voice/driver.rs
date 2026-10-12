use std::{sync::Arc, time::Instant};

use common::{
    v1::types::{
        MessageClient,
        voice::{
            IceCandidate, SessionDescription, VoiceState,
            datachannel::ProtocolType,
            messages::{SignallingCommand, SignallingEvent},
        },
    },
    v2::types::ChannelId,
};
use futures_util::{StreamExt, stream::BoxStream};
use str0m::Rtc;
use tokio::{
    net::UdpSocket,
    sync::{broadcast, mpsc},
    time,
};
use tracing::{error, info};

use crate::{syncer::SyncerHandle, voice::VoiceError};

/// sent to the worker
#[derive(Debug, Clone)]
pub(super) enum RtcCommand {
    /// handle a signalling event from the server
    Signalling(SignallingEvent),
}

pub struct Driver {
    rtc: Rtc,
    rx: mpsc::Receiver<RtcCommand>,
    syncer: SyncerHandle,
    sock: UdpSocket,
    channel_id: ChannelId,
    pending: Option<str0m::change::SdpPendingOffer>,
}

impl Driver {
    pub fn new(
        rtc: Rtc,
        rx: mpsc::Receiver<RtcCommand>,
        syncer: SyncerHandle,
        sock: UdpSocket,
        channel_id: ChannelId,
    ) -> Self {
        Driver {
            rtc,
            rx,
            syncer,
            sock,
            channel_id,
            pending: None,
        }
    }

    pub async fn spawn(mut self) {
        loop {
            if let Err(e) = self.step().await {
                error!("rtc step error: {e}");
            }
        }
    }

    pub async fn step(&mut self) -> Result<(), VoiceError> {
        if !self.rtc.is_alive() {
            // TODO: handle rtc dead
            error!("rtc dead");
            return Err(VoiceError::Dead);
        }

        let output = match self.rtc.poll_output() {
            Ok(o) => o,
            Err(e) => {
                error!("rtc poll error: {e}");
                return Err(VoiceError::Rtc(e));
            }
        };

        let timeout = match output {
            str0m::Output::Timeout(instant) => instant,
            str0m::Output::Transmit(v) => {
                self.sock.send_to(&v.contents, v.destination).await?;
                return Ok(());
            }
            str0m::Output::Event(event) => {
                self.handle_str0m_event(event).await?;
                return Ok(());
            }
        };

        let mut packet_buf = vec![0; 2048];
        let sleep = time::sleep_until(time::Instant::from_std(timeout));

        tokio::select! {
            biased;

            Some(cmd) = self.rx.recv() => {
                self.handle_command(cmd).await ?;
                return Ok(())
            },

            Ok((n, source)) = self.sock.recv_from(&mut packet_buf) => {
                let res = self.rtc.handle_input(str0m::Input::Receive(
                    Instant::now(),
                    str0m::net::Receive {
                        proto: str0m::net::Protocol::Udp,
                        source,
                        destination: self.sock.local_addr()?,
                        contents: packet_buf[..n].try_into()?,
                    },
                ));
                if let Err(e) = res {
                    error!("rtc handle_input error: {e}");
                }
            }

            _ = sleep => {
                if let Err(e) = self.rtc.handle_input(str0m::Input::Timeout(Instant::now())) {
                    error!("rtc handle_input timeout error: {e}");
                    // TODO: what now?
                }
            },
        }

        Ok(())
    }

    pub async fn handle_command(&mut self, cmd: RtcCommand) -> Result<(), VoiceError> {
        match cmd {
            RtcCommand::Signalling(s) => match s {
                SignallingEvent::Connected { .. } => {
                    info!("Connected to SFU");
                }
                SignallingEvent::Disconnected => {
                    info!("Disconnected from SFU");
                }
                SignallingEvent::Offer { sdp, .. } => {
                    let sdp = str0m::change::SdpOffer::from_sdp_string(&sdp.0)?;
                    let answer = self.rtc.sdp_api().accept_offer(sdp)?;
                    self.syncer.send(MessageClient::VoiceDispatch {
                        channel_id: self.channel_id,
                        nonce: None,
                        command: SignallingCommand::Answer {
                            sdp: SessionDescription(answer.to_sdp_string()),
                        },
                    });
                }
                SignallingEvent::Answer { sdp } => {
                    let sdp = str0m::change::SdpAnswer::from_sdp_string(&sdp.0)?;
                    if let Some(pending) = self.pending.take() {
                        self.rtc.sdp_api().accept_answer(pending, sdp)?;
                    } else {
                        error!("got answer without a pending offer, ignoring");
                    }
                }
                SignallingEvent::Candidate { candidate } => {
                    if let Ok(c) = str0m::Candidate::from_sdp_string(&candidate.0) {
                        self.rtc.add_remote_candidate(c);

                        // Send our candidate back if needed
                        // TODO: Implement ICE candidate gathering and sending
                    } else {
                        error!("failed to parse candidate: {}", candidate.0);
                    }
                }
                SignallingEvent::Tracks { .. } => {
                    // TODO: handle tracks
                }
                SignallingEvent::Subscribe(_subs) => {
                    // TODO: handle subscribe
                }
                SignallingEvent::Migrate { new_sfu_id } => {
                    info!("Migrating to SFU: {:?}", new_sfu_id);
                    // TODO: handle migrate
                }
                SignallingEvent::Error { message, code } => {
                    error!("Signalling error: {} ({:?})", message, code);
                }
            },
        }
        Ok(())
    }

    pub async fn handle_str0m_event(&mut self, event: str0m::Event) -> Result<(), VoiceError> {
        match event {
            str0m::Event::Connected => info!("rtc connected!"),
            // TODO: handle MediaAdded, MediaData
            // TODO: low priority: handle ChannelOpen, ChannelClose, ChannelData (datachannels)
            // TODO: low priority, needs thinking: handle KeyframeRequest
            _ => {}
        }

        Ok(())
    }
}
