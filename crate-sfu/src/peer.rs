use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{MediaData, PeerEvent, SignallingCommand, SignallingEvent};
use anyhow::Result;
use common::v1::types::{voice::SessionDescription, UserId};
use str0m::{
    change::{SdpAnswer, SdpOffer, SdpPendingOffer},
    media::{Direction, Mid},
    net::{Protocol, Receive},
    Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig,
};
use tokio::{
    net::UdpSocket,
    select,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::sleep_until,
};
use tracing::{debug, error, info, trace, warn};

use crate::{PeerCommand, PeerEventEnvelope, SfuTrack, TrackIn, TrackOut, TrackState};

#[derive(Debug)]
pub struct Peer {
    rtc: Rtc,
    socket: UdpSocket,
    packet: [u8; 2000],
    inbound: HashMap<Mid, TrackIn>,
    outbound: Vec<TrackOut>,
    sdp_pending: Option<SdpPendingOffer>,
    user_id: UserId,
    commands: UnboundedReceiver<PeerCommand>,
    events: UnboundedSender<PeerEventEnvelope>,
}

impl Peer {
    pub async fn spawn(
        sfu_send: UnboundedSender<PeerEventEnvelope>,
        user_id: UserId,
    ) -> Result<UnboundedSender<PeerCommand>> {
        info!("create new peer {user_id}");

        let mut rtc = RtcConfig::new()
            .set_ice_lite(true)
            .set_stats_interval(Some(Duration::from_secs(5)))
            .build();

        let addr = crate::util::select_host_address_ipv4();
        let socket = UdpSocket::bind(format!("{addr}:0")).await?;
        let candidate = Candidate::host(socket.local_addr()?, "udp")?;
        rtc.add_local_candidate(candidate);

        let (send, recv) = mpsc::unbounded_channel();

        let peer = Self {
            rtc,
            socket,
            packet: [0; 2000],
            inbound: HashMap::new(),
            outbound: vec![],
            sdp_pending: None,
            user_id,
            commands: recv,
            events: sfu_send,
        };

        tokio::spawn(async move {
            if let Err(err) = peer.run().await {
                error!("while running peer: {err:?}");
            }
        });

        Ok(send)
    }

    #[tracing::instrument(skip(self))]
    async fn run(mut self) -> Result<()> {
        loop {
            self.negotiate_if_needed()?;

            let timeout = match self.rtc.poll_output()? {
                Output::Timeout(v) => v,
                Output::Transmit(v) => {
                    trace!("transmit {} bytes to {}", v.contents.len(), v.destination);
                    self.socket.send_to(&v.contents, v.destination).await?;
                    continue;
                }
                Output::Event(v) => {
                    trace!("{v:?}");
                    match v {
                        Event::IceConnectionStateChange(IceConnectionState::Disconnected) => {
                            if dbg!(!self.rtc.is_alive()) {
                                self.rtc.disconnect();
                            }
                            break;
                        }

                        Event::Connected => debug!("connected!"),

                        Event::MediaAdded(m) => {
                            debug!("media added {m:?}");
                            self.inbound.insert(m.mid, TrackIn { _kind: m.kind });
                            self.emit(PeerEvent::MediaAdded(SfuTrack {
                                kind: m.kind,
                                mid: m.mid,
                                peer_id: self.user_id,
                            }))?;
                        }

                        Event::MediaData(m) => {
                            if let Some(_track) = self.inbound.get(&m.mid) {
                                self.emit(PeerEvent::MediaData(MediaData {
                                    mid: m.mid,
                                    peer_id: self.user_id,
                                    network_time: m.network_time,
                                    time: m.time,
                                    data: m.data.into(),
                                    params: m.params,
                                }))?;
                            } else {
                                error!("recv data with no inbound id?")
                            };
                        }

                        Event::PeerStats(_)
                        | Event::MediaIngressStats(_)
                        | Event::MediaEgressStats(_)
                        | Event::EgressBitrateEstimate(_) => {
                            debug!("{v:?}");
                        }

                        _ => {}
                    };
                    continue;
                }
            };

            let input = select! {
                _ = sleep_until(timeout.into()) => {
                    Input::Timeout(Instant::now())
                }
                recv = self.socket.recv_from(&mut self.packet) => {
                    let (n, source) = recv?;
                    Input::Receive(
                        Instant::now(),
                        Receive {
                            proto: Protocol::Udp,
                            source,
                            destination: self.socket.local_addr()?,
                            contents: self.packet[..n].try_into()?,
                        },
                    )
                }
                recv = self.commands.recv() => {
                    self.handle_sfu_command(recv.expect("channel unexpectedly closed!")).await?;
                    continue;
                },
            };

            if !self.rtc.is_alive() {
                break;
            }

            if let Err(e) = self.rtc.handle_input(input) {
                warn!("disconnected: {:?}", e);
                self.rtc.disconnect();
            }
        }

        Ok(())
    }

    async fn handle_sfu_command(&mut self, command: PeerCommand) -> Result<()> {
        match command {
            PeerCommand::Signalling(cmd) => {
                debug!("handle peer command {cmd:?}");
                self.handle_user_command(cmd).await?;
            }
            PeerCommand::MediaAdded(t) => {
                debug!("handle peer command {t:?}");

                self.outbound.push(TrackOut {
                    kind: t.kind,
                    state: TrackState::Pending,
                    peer_id: t.peer_id,
                    source_mid: t.mid,
                });
            }
            PeerCommand::MediaData(d) => self.handle_remote_media_data(d),
        }

        Ok(())
    }

    fn handle_remote_media_data(&mut self, d: MediaData) {
        let Some(mid) = self
            .outbound
            .iter()
            .find(|t| t.peer_id == d.peer_id && t.source_mid == d.mid)
            .and_then(|f| f.state.mid())
        else {
            return;
        };

        let Some(writer) = self.rtc.writer(mid) else {
            return;
        };

        let Some(pt) = writer.match_params(d.params) else {
            return;
        };

        if let Err(err) = writer.write(pt, d.network_time, d.time, d.data.to_vec()) {
            warn!("client ({}) failed: {:?}", self.user_id, err);
            self.rtc.disconnect();
        }
    }

    async fn handle_user_command(&mut self, command: SignallingCommand) -> Result<()> {
        match command {
            SignallingCommand::Answer { sdp } => self.handle_answer(sdp)?,
            SignallingCommand::Offer { sdp } => self.handle_offer(sdp)?,
            // RtcPeerCommand::IceCandidate { data } => {
            //     _ = dbg!(serde_json::from_str::<Candidate>(&data));
            // }
            _ => {}
        }
        Ok(())
    }

    fn handle_answer(&mut self, sdp: SessionDescription) -> Result<()> {
        if let Some(pending) = self.sdp_pending.take() {
            let answer = SdpAnswer::from_sdp_string(&sdp)?;
            self.rtc.sdp_api().accept_answer(pending, answer)?;

            for track in &mut self.outbound {
                if let TrackState::Negotiating(m) = track.state {
                    track.state = TrackState::Open(m);
                }
            }
        }

        Ok(())
    }

    fn handle_offer(&mut self, sdp: SessionDescription) -> Result<()> {
        let offer = SdpOffer::from_sdp_string(&sdp)?;
        let answer = self.rtc.sdp_api().accept_offer(offer)?;

        // renegotiate
        for track in &mut self.outbound {
            if let TrackState::Negotiating(_) = track.state {
                track.state = TrackState::Pending;
            }
        }

        self.emit(PeerEvent::Signalling(SignallingEvent::Answer {
            sdp: answer.to_sdp_string(),
        }))?;

        Ok(())
    }

    fn negotiate_if_needed(&mut self) -> Result<bool> {
        if self.sdp_pending.is_some() {
            return Ok(false);
        }

        let mut change = self.rtc.sdp_api();

        for track in &mut self.outbound {
            if track.state == TrackState::Pending {
                let mid = change.add_media(track.kind, Direction::SendOnly, None, None, None);
                track.state = TrackState::Negotiating(mid);
            }
        }
        if !change.has_changes() {
            return Ok(false);
        }

        let Some((offer, pending)) = change.apply() else {
            return Ok(false);
        };

        self.sdp_pending = Some(pending);
        self.emit(PeerEvent::Signalling(SignallingEvent::Offer {
            sdp: offer.to_sdp_string(),
        }))?;

        Ok(true)
    }

    fn emit(&self, event: PeerEvent) -> Result<()> {
        self.events.send(PeerEventEnvelope {
            user_id: self.user_id,
            payload: event,
        })?;
        Ok(())
    }
}
