use std::{collections::HashMap, net::SocketAddr, time::Instant};

use crate::{config::Config, MediaData, PeerEvent, SfuTrack, SignallingMessage};
use anyhow::Result;
use common::v1::types::{
    voice::{MediaKindSerde, SessionDescription, TrackMetadata, VoiceState},
    UserId,
};
use str0m::{
    change::{SdpAnswer, SdpOffer, SdpPendingOffer},
    media::{Direction, MediaKind, Mid},
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

use crate::{PeerCommand, PeerEventEnvelope, TrackIn, TrackOut, TrackState};

#[derive(Debug)]
pub struct Peer {
    rtc: Rtc,
    socket_v4: UdpSocket,
    socket_v6: UdpSocket,
    packet_v4: [u8; 2000],
    packet_v6: [u8; 2000],

    /// media data we are recieving from the user
    inbound: HashMap<Mid, TrackIn>,

    /// media data we are sending to the user
    outbound: Vec<TrackOut>,

    // outbound: HashMap<Mid, TrackOut>,
    sdp_pending: Option<SdpPendingOffer>,
    user_id: UserId,
    voice_state: VoiceState,
    commands: UnboundedReceiver<PeerCommand>,
    events: UnboundedSender<PeerEventEnvelope>,
}

impl Peer {
    pub async fn spawn(
        config: &Config,
        sfu_send: UnboundedSender<PeerEventEnvelope>,
        user_id: UserId,
        voice_state: VoiceState,
    ) -> Result<UnboundedSender<PeerCommand>> {
        info!("create new peer {user_id}");

        let mut rtc = RtcConfig::new()
            .set_ice_lite(true)
            // .set_stats_interval(Some(Duration::from_secs(5)))
            .build();

        let addr = crate::util::select_host_address_ipv4(config.host_ipv4.as_deref());
        let socket_v4 = UdpSocket::bind(format!("{addr}:0")).await?;
        let candidate = Candidate::host(socket_v4.local_addr()?, "udp")?;
        debug!("listen on {}", socket_v4.local_addr().unwrap());
        rtc.add_local_candidate(candidate.clone());

        let addr = crate::util::select_host_address_ipv6(config.host_ipv6.as_deref());
        let socket_v6 = UdpSocket::bind(format!("[{addr}]:0")).await?;
        let candidate = Candidate::host(socket_v6.local_addr()?, "udp")?;
        debug!("listen on {}", socket_v6.local_addr().unwrap());
        rtc.add_local_candidate(candidate.clone());

        let (send, recv) = mpsc::unbounded_channel();

        let mut peer = Self {
            rtc,
            socket_v4,
            socket_v6,
            inbound: HashMap::new(),
            outbound: vec![],
            sdp_pending: None,
            user_id,
            voice_state,
            commands: recv,
            events: sfu_send,
            packet_v4: [0; 2000],
            packet_v6: [0; 2000],
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

                    match v.destination {
                        SocketAddr::V4(_) => {
                            self.socket_v4.send_to(&v.contents, v.destination).await?;
                        }
                        SocketAddr::V6(_) => {
                            self.socket_v6.send_to(&v.contents, v.destination).await?;
                        }
                    }
                    continue;
                }
                Output::Event(v) => {
                    trace!("{v:?}");
                    match v {
                        Event::Connected => debug!("connected!"),

                        Event::MediaAdded(m) => {
                            // TODO: enforce max bitrate, resolution
                            debug!("media added {m:?}");

                            let mid = m.mid;

                            let mut events = vec![];
                            if let Some(track) = self.inbound.get_mut(&mid) {
                                if let TrackState::Negotiating(_) = track.state {
                                    track.state = TrackState::Open(mid);

                                    events.push(PeerEvent::MediaAdded(SfuTrack {
                                        source_mid: mid,
                                        kind: track.kind,
                                        peer_id: self.user_id,
                                        key: track.key.clone(),
                                        thread_id: track.thread_id,
                                    }));

                                    events.push(PeerEvent::Signalling(SignallingMessage::Have {
                                        user_id: self.user_id,
                                        thread_id: track.thread_id,
                                        tracks: vec![TrackMetadata {
                                            mid: mid.to_string(),
                                            kind: match track.kind {
                                                MediaKind::Audio => MediaKindSerde::Audio,
                                                MediaKind::Video => MediaKindSerde::Video,
                                            },
                                            key: track.key.clone(),
                                        }],
                                    }));
                                }
                            } else {
                                // self.inbound.insert(
                                //     mid,
                                //     TrackIn {
                                //         kind: m.kind,
                                //         state: TrackState::Open(mid),
                                //         thread_id: self.voice_state.thread_id,
                                //         key: todo!(),
                                //     },
                                // );
                            }

                            for event in events {
                                self.emit(event)?;
                            }
                        }

                        Event::MediaData(m) => self.handle_media_data(m)?,

                        Event::KeyframeRequest(r) => {
                            let track = self
                                .outbound
                                .iter()
                                .find(|t| t.state == TrackState::Open(r.mid));
                            if let Some(track) = track {
                                self.emit(PeerEvent::NeedsKeyframe {
                                    source_mid: track.source_mid,
                                    source_peer: track.peer_id,
                                    for_peer: self.user_id,
                                    kind: r.kind,
                                    rid: r.rid,
                                })?;
                            } else {
                                warn!("track not found");
                            }
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
                recv = self.socket_v4.recv_from(&mut self.packet_v4) => {
                    let (n, source) = recv?;
                    Input::Receive(
                        Instant::now(),
                        Receive {
                            proto: Protocol::Udp,
                            source,
                            destination: self.socket_v4.local_addr()?,
                            contents: self.packet_v4[..n].try_into()?,
                        },
                    )
                }
                recv = self.socket_v6.recv_from(&mut self.packet_v6) => {
                    let (n, source) = recv?;
                    Input::Receive(
                        Instant::now(),
                        Receive {
                            proto: Protocol::Udp,
                            source,
                            destination: self.socket_v6.local_addr()?,
                            contents: self.packet_v6[..n].try_into()?,
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
                self.handle_signalling(cmd).await?;
            }
            PeerCommand::MediaAdded(t) => {
                debug!("handle peer command {t:?}");

                self.outbound.push(TrackOut {
                    kind: t.kind,
                    state: TrackState::Pending,
                    peer_id: t.peer_id,
                    source_mid: t.source_mid,
                    enabled: false,
                    thread_id: t.thread_id,
                    key: t.key,
                });
            }
            PeerCommand::MediaData(d) => self.handle_remote_media_data(d),
            PeerCommand::Kill => self.rtc.disconnect(),
            PeerCommand::GenerateKeyframe {
                mid,
                kind,
                for_peer: _, // do i need this? how do i use this?
                rid,
            } => {
                let Some(mut writer) = self.rtc.writer(mid) else {
                    debug!("track has no writer");
                    return Ok(());
                };

                if let Err(err) = writer.request_keyframe(rid, kind) {
                    warn!("failed to generate keyframe: {:?}", err);
                }
            }
        }

        Ok(())
    }

    fn handle_remote_media_data(&mut self, d: MediaData) {
        debug!("handle_remote_media_data");

        let Some(track) = self
            .outbound
            .iter_mut()
            .find(|t| t.peer_id == d.peer_id && t.source_mid == d.mid)
        else {
            debug!("track has no outbound entry");
            return;
        };

        // if !track.enabled {
        //     return;
        // }

        let Some(mid) = track.state.mid() else {
            debug!("track has no mid");
            return;
        };

        let Some(writer) = self.rtc.writer(mid) else {
            debug!("track has no writer");
            return;
        };

        let Some(pt) = writer.match_params(d.params) else {
            debug!("track has no payload type");
            return;
        };

        if let Err(err) = writer.write(pt, d.network_time, d.time, d.data.as_ref()) {
            warn!("client ({}) failed: {:?}", self.user_id, err);
            self.rtc.disconnect();
        }
    }

    async fn handle_signalling(&mut self, command: SignallingMessage) -> Result<()> {
        debug!("signalling {command:?}");
        match command {
            SignallingMessage::Answer { sdp } => self.handle_answer(sdp)?,
            SignallingMessage::Offer { sdp, tracks } => self.handle_offer(sdp, tracks)?,
            SignallingMessage::Candidate { candidate } => {
                if let Ok(candidate) = Candidate::from_sdp_string(&candidate) {
                    self.rtc.add_remote_candidate(candidate);
                } else {
                    warn!("invalid candidate: {candidate:?}")
                }
            }
            SignallingMessage::Want { tracks: _ } => todo!(),
            SignallingMessage::Have { .. } => panic!("server only"),
            SignallingMessage::VoiceState { state } => {
                self.voice_state.thread_id = state.unwrap().thread_id;
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

            // let meta = self.inbound.iter().map(|(mid, a)| TrackMetadata {
            //     mid: mid.to_string(),
            //     kind: match a.kind {
            //         str0m::media::MediaKind::Audio => MediaKindSerde::Audio,
            //         str0m::media::MediaKind::Video => MediaKindSerde::Video,
            //     },
            //     key: a.key.clone(),
            // }).collect();

            // for track in self.inbound.values_mut() {
            //     if let TrackState::Negotiating(m) = track.state {
            //         track.state = TrackState::Open(m);
            //         events.push(PeerEvent::Signalling(SignallingMessage::Have {
            //             user_id: self.user_id,
            //             thread_id: self.voice_state.thread_id,
            //             tracks: vec![TrackMetadata {
            //                 mid: m.to_string(),
            //                 kind: match track.kind {
            //                     str0m::media::MediaKind::Audio => MediaKindSerde::Audio,
            //                     str0m::media::MediaKind::Video => MediaKindSerde::Video,
            //                 },
            //                 key: track.key.to_owned(),
            //             }],
            //         }));
            //     }
            // }
        }

        Ok(())
    }

    fn handle_offer(&mut self, sdp: SessionDescription, tracks: Vec<TrackMetadata>) -> Result<()> {
        let offer = SdpOffer::from_sdp_string(&sdp)?;
        let answer = self.rtc.sdp_api().accept_offer(offer)?;

        // renegotiate
        for track in &mut self.outbound {
            if let TrackState::Negotiating(_) = track.state {
                track.state = TrackState::Pending;
            }
        }

        // TODO: send clear events/track list to other peers
        // TODO: verify that this does what i think it does
        self.inbound.clear();
        for track in tracks {
            let mid = Mid::from(track.mid.as_str());
            self.inbound.insert(
                mid,
                TrackIn {
                    kind: match track.kind {
                        MediaKindSerde::Video => MediaKind::Video,
                        MediaKindSerde::Audio => MediaKind::Audio,
                    },
                    state: TrackState::Negotiating(mid),
                    thread_id: self.voice_state.thread_id,
                    key: track.key,
                },
            );
        }

        self.sdp_pending = None;
        self.emit(PeerEvent::Signalling(SignallingMessage::Answer {
            sdp: SessionDescription(answer.to_sdp_string()),
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
        self.emit(PeerEvent::Signalling(SignallingMessage::Offer {
            sdp: SessionDescription(offer.to_sdp_string()),
            tracks: vec![],
        }))?;

        Ok(true)
    }

    fn handle_media_data(&self, data: str0m::media::MediaData) -> Result<()> {
        let Some(track) = self.inbound.get(&data.mid) else {
            debug!("no inbound track");
            return Ok(());
        };

        if !matches!(track.state, TrackState::Open(_)) {
            debug!("track not open");
            return Ok(());
        };

        debug!("emit media data");
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
}
