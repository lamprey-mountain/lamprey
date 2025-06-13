use std::{collections::HashMap, time::Instant};

use crate::{MediaData, PeerEvent, SignallingCommand, SignallingEvent};
use anyhow::Result;
use common::v1::types::{voice::SessionDescription, UserId};
use str0m::{
    change::{SdpAnswer, SdpOffer, SdpPendingOffer},
    media::{Direction, KeyframeRequestKind, Mid},
    net::{Protocol, Receive},
    Candidate, Event, Input, Output, Rtc, RtcConfig,
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
            // .set_stats_interval(Some(Duration::from_secs(5)))
            .build();

        let addr = crate::util::select_host_address_ipv4();
        let socket = UdpSocket::bind(format!("{addr}:0")).await?;
        let candidate = Candidate::host(socket.local_addr()?, "udp")?;
        rtc.add_local_candidate(candidate);

        let (send, recv) = mpsc::unbounded_channel();

        let mut peer = Self {
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
            debug!("dead!");
            _ = peer.emit(PeerEvent::Dead);
        });

        Ok(send)
    }

    #[tracing::instrument(skip(self))]
    async fn run(&mut self) -> Result<()> {
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
                        Event::Connected => debug!("connected!"),

                        Event::MediaAdded(m) => {
                            debug!("media added {m:?}");
                            self.inbound.insert(
                                m.mid,
                                TrackIn {
                                    kind: m.kind,
                                    state: TrackState::Negotiating(m.mid),
                                },
                            );
                            self.emit(PeerEvent::MediaAdded(SfuTrack {
                                kind: m.kind,
                                mid: m.mid,
                                peer_id: self.user_id,
                                key: None,
                            }))?;
                        }

                        Event::MediaData(m) => self.handle_media_data(m)?,

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
                    if let Some(recv) = recv {
                        self.handle_sfu_command(recv).await?;
                    } else {
                        self.rtc.disconnect();
                    }
                    if !self.rtc.is_alive() {
                        break;
                    }
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
                    enabled: false,
                    needs_keyframe: false,
                });
            }
            PeerCommand::MediaData(d) => self.handle_remote_media_data(d),
            PeerCommand::Kill => self.rtc.disconnect(),
            PeerCommand::RemotePublish { user_id, mid, key } => {
                self.publish(user_id, mid, key)?;
            }
        }

        Ok(())
    }

    fn handle_remote_media_data(&mut self, d: MediaData) {
        let Some(track) = self
            .outbound
            .iter_mut()
            .find(|t| t.peer_id == d.peer_id && t.source_mid == d.mid)
        else {
            return;
        };

        if !track.enabled {
            return;
        }

        let Some(mid) = track.state.mid() else {
            return;
        };

        let Some(mut writer) = self.rtc.writer(mid) else {
            return;
        };

        let Some(pt) = writer.match_params(d.params) else {
            return;
        };

        // FIXME: keyframe requests
        // if track.needs_keyframe {
        //     if let Err(err) = writer.request_keyframe(None, KeyframeRequestKind::Pli) {
        //         warn!("failed to generate keyframe: {:?}", err);
        //     } else {
        //         track.needs_keyframe = false;
        //     }
        // }

        if let Err(err) = writer.write(pt, d.network_time, d.time, d.data.to_vec()) {
            warn!("client ({}) failed: {:?}", self.user_id, err);
            self.rtc.disconnect();
        }
    }

    async fn handle_user_command(&mut self, command: SignallingCommand) -> Result<()> {
        match command {
            SignallingCommand::Answer { sdp } => self.handle_answer(sdp)?,
            SignallingCommand::Offer { sdp } => self.handle_offer(sdp)?,
            SignallingCommand::VoiceState { .. } => {}
            SignallingCommand::Publish { mid, key } => {
                info!("got publish mid={mid} key={key}");
                let mid = Mid::from(mid.as_str());
                if let Some((_, track)) = self.inbound.iter_mut().find(|t| t.0 == &mid) {
                    if matches!(track.state, TrackState::Negotiating(_)) {
                        track.state = TrackState::Open(mid);
                        self.emit(PeerEvent::Signalling(SignallingEvent::Subscribe {
                            mid: mid.to_string(),
                        }))?;
                    } else {
                        error!("media already published")
                    }
                } else {
                    error!("media not found")
                }
            }
            SignallingCommand::Subscribe { mid } => {
                info!("got subscribe mid={mid}");
                let mid = Mid::from(mid.as_str());
                if let Some(track) = self
                    .outbound
                    .iter_mut()
                    .find(|t| t.state == TrackState::Open(mid))
                {
                    debug!("enabling track");
                    track.enabled = true;
                    track.needs_keyframe = true;
                } else {
                    error!("media not found")
                }
            }
            SignallingCommand::Unsubscribe { mid } => {
                info!("got unsubscribe mid={mid}");
                let mid = Mid::from(mid.as_str());
                if let Some(track) = self
                    .outbound
                    .iter_mut()
                    .find(|t| t.state == TrackState::Open(mid))
                {
                    track.enabled = false;
                } else {
                    error!("media not found")
                }
            }
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

        self.sdp_pending = None;
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
                let mid = change.add_media(
                    track.kind,
                    Direction::SendOnly,
                    // Some(track.ssrc.clone()),
                    None,
                    None,
                    None,
                );
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

    fn handle_media_data(&self, data: str0m::media::MediaData) -> Result<()> {
        let Some(track) = self.inbound.get(&data.mid) else {
            return Ok(());
        };

        if !matches!(track.state, TrackState::Open(_)) {
            return Ok(());
        };

        self.emit(PeerEvent::MediaData(MediaData {
            mid: data.mid,
            peer_id: self.user_id,
            network_time: data.network_time,
            time: data.time,
            data: data.data.into(),
            params: data.params,
        }))?;

        Ok(())
    }

    fn emit(&self, event: PeerEvent) -> Result<()> {
        self.events.send(PeerEventEnvelope {
            user_id: self.user_id,
            payload: event,
        })?;
        Ok(())
    }

    fn publish(&self, user_id: UserId, source_mid: Mid, key: String) -> Result<()> {
        if let Some(t) = self
            .outbound
            .iter()
            .find(|t| t.source_mid == source_mid && t.peer_id == user_id)
        {
            debug!("found track {:?}", t.state);
            match t.state {
                TrackState::Open(mid) | TrackState::Negotiating(mid) => {
                    self.emit(PeerEvent::Signalling(SignallingEvent::Publish {
                        user_id,
                        mid: mid.to_string(),
                        key,
                    }))?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
