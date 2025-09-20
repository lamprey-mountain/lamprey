use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};

use crate::{
    config::Config, signalling::Signalling, Error, MediaData, PeerEvent, SignallingMessage,
    TrackMetadataServer, TrackMetadataSfu,
};
use anyhow::Result;
use common::v1::types::{
    voice::{
        SessionDescription, Speaking, SpeakingWithoutUserId, TrackId, TrackMetadata, VoiceState,
    },
    UserId,
};
use str0m::{
    channel::ChannelId,
    media::{Direction, Mid},
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
    signalling: Signalling,

    /// media data we are receiving from the user
    inbound: HashMap<Mid, TrackIn>,

    /// media data we are sending to the user
    outbound: Vec<TrackOut>,

    /// metadata for each track we are receiving from the user
    tracks_metadata: Vec<TrackMetadataServer>,

    /// we want Have messages for media from these users
    have_queue: Vec<UserId>,

    user_id: UserId,
    voice_state: VoiceState,
    commands: UnboundedReceiver<PeerCommand>,
    events: UnboundedSender<PeerEventEnvelope>,

    /// the datachannel where speaking data is sent
    speaking_chan: Option<ChannelId>,
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
            .set_stats_interval(Some(Duration::from_secs(5)))
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
            user_id,
            voice_state,
            commands: recv,
            events: sfu_send,
            packet_v4: [0; 2000],
            packet_v6: [0; 2000],
            tracks_metadata: vec![],
            have_queue: vec![],
            signalling: Signalling::new(),
            speaking_chan: None,
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

    #[tracing::instrument(skip(self), fields(user_id = %self.user_id))]
    async fn run(&mut self) -> Result<()> {
        loop {
            self.negotiate_if_needed()?;

            let timeout = match self.rtc.poll_output()? {
                Output::Timeout(v) => v,
                Output::Transmit(v) => {
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
                    self.handle_event(v).await?;
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

    async fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Connected => debug!("connected!"),

            Event::MediaAdded(m) => {
                debug!("media added {m:?}");
                self.register_media(m.mid)?;
            }

            Event::MediaChanged(m) => {
                debug!("media changed {m:?}");
                self.register_media(m.mid)?;
            }

            Event::MediaData(m) => self.handle_media_data(m)?,

            Event::KeyframeRequest(r) => {
                debug!("keyframe request {r:?}");
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

            Event::ChannelOpen(chan, label) => {
                if label == "speaking" {
                    debug!("open speaking channel {chan:?}");
                    self.speaking_chan = Some(chan);
                }
            }
            Event::ChannelData(data) => {
                if self.speaking_chan == Some(data.id) {
                    if let Ok(data) = serde_json::from_slice::<SpeakingWithoutUserId>(&data.data) {
                        debug!("recv speaking {data:?}");
                        self.emit(PeerEvent::Speaking(Speaking {
                            user_id: self.user_id,
                            flags: data.flags,
                        }))?;
                    } else {
                        debug!("recv speaking invalid data");
                    }
                }
            }

            Event::PeerStats(_)
            | Event::MediaIngressStats(_)
            | Event::MediaEgressStats(_)
            | Event::EgressBitrateEstimate(_) => {
                debug!("{event:?}");
            }

            _ => {
                trace!("{event:?}");
            }
        };

        Ok(())
    }

    /// handle a command from the sfu
    async fn handle_sfu_command(&mut self, command: PeerCommand) -> Result<()> {
        match command {
            PeerCommand::Signalling(cmd) => {
                self.handle_signalling(cmd).await?;
            }
            PeerCommand::MediaAdded(t) => {
                debug!("handle media added {t:?}");

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
            PeerCommand::Have { user_id, tracks } => {
                if !self.process_haves(user_id, tracks)? {
                    self.have_queue.push(user_id);
                }
            }
            PeerCommand::Speaking(speaking) => {
                let chan = self.speaking_chan.and_then(|ch| self.rtc.channel(ch));
                if let Some(mut chan) = chan {
                    chan.write(false, &serde_json::to_vec(&speaking)?)?;
                }
            }
        }

        Ok(())
    }

    /// map tracks from these users to local outbound tracks, then send a Have message for them
    fn process_haves(&mut self, user_id: UserId, tracks: Vec<TrackMetadataServer>) -> Result<bool> {
        let mut out = vec![];
        for t in tracks {
            let our_track = self
                .outbound
                .iter()
                .find(|a| a.peer_id == user_id && a.source_mid == t.source_mid);
            if let Some(a) = our_track {
                if let TrackState::Open(mid) = a.state {
                    out.push(TrackMetadata {
                        mid: TrackId(mid.to_string()),
                        kind: t.kind,
                        key: t.key,
                    });
                } else {
                    warn!("track not open (peer_id={}, track={:?})", user_id, t);
                    return Ok(false);
                }
            } else {
                warn!("missing track (peer_id={}, track={:?})", user_id, t);
                return Ok(false);
            }
        }
        self.emit(PeerEvent::Signalling(SignallingMessage::Have {
            thread_id: self.voice_state.thread_id,
            user_id,
            tracks: out,
        }))?;
        Ok(true)
    }

    /// handle media data from a remote peer
    fn handle_remote_media_data(&mut self, d: MediaData) {
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

    /// handle a signalling message from the peer
    async fn handle_signalling(&mut self, command: SignallingMessage) -> Result<()> {
        debug!("signalling {command:?}");
        match command {
            SignallingMessage::Answer { sdp } => self.handle_answer(sdp)?,
            SignallingMessage::Offer { sdp, tracks } => self.handle_offer(sdp, tracks)?,
            SignallingMessage::Candidate { candidate } => {
                // str0m only supports some candidates right now, not sure if this causes problems
                if let Ok(candidate) = Candidate::from_sdp_string(&candidate) {
                    self.rtc.add_remote_candidate(candidate);
                }
            }
            SignallingMessage::Want { tracks: _ } => todo!(),
            SignallingMessage::Have { .. } => return Err(Error::HaveServerOnly.into()),
            SignallingMessage::Reconnect => panic!("handled by sfu"),
            SignallingMessage::VoiceState { state } => {
                self.voice_state.thread_id = state.unwrap().thread_id;
            }
            SignallingMessage::Ready => {}
        }
        Ok(())
    }

    /// handle an sdp answer from the peer
    fn handle_answer(&mut self, sdp: SessionDescription) -> Result<()> {
        self.signalling.handle_answer(&mut self.rtc, sdp)?;

        for track in &mut self.outbound {
            if let TrackState::Negotiating(m) = track.state {
                track.state = TrackState::Open(m);
            }
        }

        let user_ids = std::mem::take(&mut self.have_queue);
        self.emit(PeerEvent::WantHave { user_ids })?;

        Ok(())
    }

    /// handle an sdp offer from the peer
    fn handle_offer(&mut self, sdp: SessionDescription, tracks: Vec<TrackMetadata>) -> Result<()> {
        let answer = self.signalling.handle_offer(&mut self.rtc, sdp)?;

        // renegotiate outbound tracks
        for track in &mut self.outbound {
            if let TrackState::Negotiating(_) = track.state {
                track.state = TrackState::Pending;
            }
        }

        let inbound_old = std::mem::take(&mut self.inbound);
        for track in tracks {
            let mid = Mid::from(&*track.mid);
            let state = inbound_old
                .get(&mid)
                .map(|t| t.state)
                .unwrap_or(TrackState::Negotiating(mid));
            self.inbound.insert(
                mid,
                TrackIn {
                    kind: track.kind.into(),
                    state,
                    thread_id: self.voice_state.thread_id,
                    key: track.key,
                },
            );
        }

        // HACK: manually disable set port to 0 to disable media when the answer has no codecs
        let sdp_str = answer
            .to_sdp_string()
            .lines()
            .map(|line| {
                if line.starts_with("m=") {
                    let mut parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() == 3 {
                        parts[1] = "0";
                        return parts.join(" ");
                    }
                }
                line.to_string()
            })
            .collect::<Vec<String>>()
            .join("\r\n")
            + "\r\n";

        self.emit(PeerEvent::Signalling(SignallingMessage::Answer {
            sdp: SessionDescription(sdp_str),
        }))?;

        let user_ids = std::mem::take(&mut self.have_queue);
        self.emit(PeerEvent::WantHave { user_ids })?;

        Ok(())
    }

    /// send an sdp offer if we have tracks that haven't been negotiated yet
    fn negotiate_if_needed(&mut self) -> Result<()> {
        let mut change = self.rtc.sdp_api();

        // create pending outbound tracks
        for track in &mut self.outbound {
            if track.state == TrackState::Pending {
                let mid = change.add_media(track.kind, Direction::SendOnly, None, None, None);
                track.state = TrackState::Negotiating(mid);
            }
        }

        if let Some(offer) = self.signalling.negotiate_if_needed(change)? {
            self.emit(PeerEvent::Signalling(SignallingMessage::Offer {
                sdp: SessionDescription(offer.to_sdp_string()),
                tracks: vec![],
            }))?;
        }

        Ok(())
    }

    /// handle media data from the local peer
    fn handle_media_data(&self, data: str0m::media::MediaData) -> Result<()> {
        let Some(track) = self.inbound.get(&data.mid) else {
            debug!("no inbound track");
            return Ok(());
        };

        if !matches!(track.state, TrackState::Open(_)) {
            debug!("track not open");
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

    fn register_media(&mut self, mid: Mid) -> Result<()> {
        let mut events = vec![];
        let mut tracks_metadata = vec![];

        if let Some(track) = self.inbound.get_mut(&mid) {
            if let TrackState::Negotiating(_) = track.state {
                track.state = TrackState::Open(mid);

                events.push(PeerEvent::MediaAdded(TrackMetadataSfu {
                    source_mid: mid,
                    kind: track.kind,
                    peer_id: self.user_id,
                    key: track.key.clone(),
                    thread_id: track.thread_id,
                }));

                tracks_metadata.push(TrackMetadataServer {
                    source_mid: mid,
                    kind: track.kind.into(),
                    key: track.key.clone(),
                });
            } else {
                warn!(
                    "MediaAdded event for mid {mid} track state is {:?}",
                    track.state
                );
            }
        } else {
            warn!("MediaAdded event for mid {mid} we don't have the track metadata");
        }

        if !tracks_metadata.is_empty() {
            self.tracks_metadata.append(&mut tracks_metadata);

            events.push(PeerEvent::Have {
                tracks: self.tracks_metadata.clone(),
            });

            events.push(PeerEvent::Signalling(SignallingMessage::Have {
                user_id: self.user_id,
                thread_id: self.voice_state.thread_id,
                tracks: self
                    .tracks_metadata
                    .iter()
                    .map(|t| TrackMetadata {
                        mid: TrackId(t.source_mid.to_string()),
                        kind: t.kind,
                        key: t.key.clone(),
                    })
                    .collect(),
            }));
        }

        for event in events {
            self.emit(event)?;
        }

        Ok(())
    }

    /// send an event to the sfu
    fn emit(&self, event: PeerEvent) -> Result<()> {
        self.events.send(PeerEventEnvelope {
            user_id: self.user_id,
            payload: event,
        })?;
        Ok(())
    }
}
