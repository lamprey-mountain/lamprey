use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    config::Config, signalling::Signalling, Error, MediaData, PeerEvent, PeerPermissions,
    SignallingMessage, TrackMetadataServer, TrackMetadataSfu,
};
use anyhow::Result;
use bytes::Bytes;
use common::v1::types::{
    voice::{
        SessionDescription, SfuPermissions, Speaking, SpeakingWithoutUserId, TrackId, TrackKey,
        TrackMetadata, VoiceState,
    },
    UserId,
};
use str0m::{
    channel::ChannelId,
    media::{Direction, MediaKind, Mid},
    Candidate, Event, Input, Output, Rtc, RtcConfig,
};
use tokio::{
    net::UdpSocket,
    select,
    sync::mpsc::{self, Receiver, UnboundedReceiver, UnboundedSender},
    time::sleep_until,
};
use tracing::{debug, error, info, trace, warn};

use crate::{PeerCommand, PeerEventEnvelope, PeerMedia, TrackIn, TrackOut, TrackState};

#[derive(Debug)]
pub struct Peer {
    rtc: Rtc,
    socket_v4: Arc<UdpSocket>,
    socket_v6: Arc<UdpSocket>,
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
    permissions: PeerPermissions,
    commands: UnboundedReceiver<PeerCommand>,
    media_rx: Receiver<PeerMedia>,
    events: UnboundedSender<PeerEventEnvelope>,

    /// the datachannel where speaking data is sent
    speaking_chan: Option<ChannelId>,

    /// direct routing table to other peers (the data plane)
    routing_table: HashMap<UserId, (UnboundedSender<PeerCommand>, mpsc::Sender<PeerMedia>)>,

    /// packets from the shared worker socket
    packet_rx: UnboundedReceiver<(SocketAddr, Bytes)>,

    last_ufrag: Option<String>,
}

impl Peer {
    pub async fn create(
        config: &Config,
        sfu_send: UnboundedSender<PeerEventEnvelope>,
        user_id: UserId,
        voice_state: VoiceState,
        permissions: SfuPermissions,
        socket_v4: Arc<UdpSocket>,
        socket_v6: Arc<UdpSocket>,
        packet_rx: UnboundedReceiver<(SocketAddr, Bytes)>,
    ) -> Result<(Self, UnboundedSender<PeerCommand>, mpsc::Sender<PeerMedia>)> {
        info!("create new peer {user_id}");

        let mut rtc_config = RtcConfig::new()
            .set_ice_lite(true)
            .set_stats_interval(Some(Duration::from_secs(5)))
            .build();

        // Local candidate is now the shared worker port
        let addr_v4 = crate::util::select_host_address_ipv4(config.host_ipv4.as_deref())?;
        let candidate_v4 = Candidate::host(SocketAddr::new(addr_v4, config.udp_port), "udp")?;
        rtc_config.add_local_candidate(candidate_v4);

        let addr_v6 = crate::util::select_host_address_ipv6(config.host_ipv6.as_deref())?;
        let candidate_v6 = Candidate::host(SocketAddr::new(addr_v6, config.udp_port), "udp")?;
        rtc_config.add_local_candidate(candidate_v6);

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (media_tx, media_rx) = mpsc::channel(128); // Bounded media channel

        let peer = Self {
            rtc: rtc_config,
            socket_v4,
            socket_v6,
            inbound: HashMap::new(),
            outbound: vec![],
            user_id,
            voice_state,
            permissions: permissions.into(),
            commands: cmd_rx,
            media_rx,
            events: sfu_send,
            tracks_metadata: vec![],
            have_queue: vec![],
            signalling: Signalling::new(),
            speaking_chan: None,
            routing_table: HashMap::new(),
            packet_rx,
            last_ufrag: None,
        };

        Ok((peer, cmd_tx, media_tx))
    }

    pub async fn run_loop(mut self) {
        if let Err(err) = self.run().await {
            error!("while running peer {}: {err:?}", self.user_id);
        }
        debug!("dead! {}", self.user_id);
        _ = self.emit(PeerEvent::Dead);
    }

    #[tracing::instrument(skip(self), fields(user_id = %self.user_id))]
    async fn run(&mut self) -> Result<()> {
        loop {
            self.negotiate_if_needed()?;

            let ufrag = self
                .rtc
                .direct_api()
                .local_ice_credentials()
                .ufrag
                .to_string();
            if self.last_ufrag.as_ref() != Some(&ufrag) {
                self.last_ufrag = Some(ufrag.clone());
                _ = self.emit(PeerEvent::IceUfrag(ufrag));
            }

            let timeout = match self.rtc.poll_output()? {
                Output::Timeout(v) => v,
                Output::Transmit(v) => {
                    match v.destination {
                        SocketAddr::V4(_) => {
                            _ = self.socket_v4.send_to(&v.contents, v.destination).await;
                        }
                        SocketAddr::V6(_) => {
                            _ = self.socket_v6.send_to(&v.contents, v.destination).await;
                        }
                    }
                    continue;
                }
                Output::Event(v) => {
                    self.handle_event(v).await?;
                    continue;
                }
            };

            let input_data = select! {
                _ = sleep_until(timeout.into()) => {
                    None
                }
                recv = self.packet_rx.recv() => {
                    if let Some((source, data)) = recv {
                        Some((source, data))
                    } else {
                        break; // worker closed our packet channel
                    }
                }
                recv = self.media_rx.recv() => {
                    if let Some(media) = recv {
                        self.handle_remote_media(media).await?;
                    }
                    continue;
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

            let input = if let Some((source, data)) = &input_data {
                str0m::Input::Receive(
                    Instant::now(),
                    str0m::net::Receive {
                        proto: str0m::net::Protocol::Udp,
                        source: *source,
                        destination: if source.is_ipv4() {
                            self.socket_v4.local_addr().unwrap()
                        } else {
                            self.socket_v6.local_addr().unwrap()
                        },
                        contents: data.as_ref().try_into().unwrap(),
                    },
                )
            } else {
                Input::Timeout(Instant::now())
            };

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
                        let speaking = Speaking {
                            user_id: self.user_id,
                            flags: data.flags,
                        };

                        // Data Plane: broadcast directly to all other peers
                        for (other_id, (_, media_tx)) in &self.routing_table {
                            if let Err(e) = media_tx.try_send(PeerMedia::Speaking(speaking.clone()))
                            {
                                trace!("failed to send speaking to peer {other_id}: {e}");
                            }
                        }

                        // Also emit to SFU for signaling-level tracking
                        self.emit(PeerEvent::Speaking(speaking))?;
                    } else {
                        debug!("recv speaking invalid data");
                    }
                }
            }

            _ => {}
        };

        Ok(())
    }

    async fn handle_remote_media(&mut self, media: PeerMedia) -> Result<()> {
        match media {
            PeerMedia::MediaData(d) => self.handle_remote_media_data(d),
            PeerMedia::Speaking(speaking) => {
                let chan = self.speaking_chan.and_then(|ch| self.rtc.channel(ch));
                if let Some(mut chan) = chan {
                    _ = chan.write(false, &serde_json::to_vec(&speaking)?);
                }
            }
        }
        Ok(())
    }

    /// handle a command from the sfu
    async fn handle_sfu_command(&mut self, command: PeerCommand) -> Result<()> {
        match command {
            PeerCommand::UpdateRoutingTable {
                user_id,
                signalling_sender,
                media_sender,
            } => {
                self.routing_table
                    .insert(user_id, (signalling_sender, media_sender));
            }
            PeerCommand::Signalling(cmd) => {
                self.handle_signalling(cmd).await?;
            }
            PeerCommand::Speaking(media) => {
                self.handle_remote_media(media).await?;
            }
            PeerCommand::VoiceState(state) => {
                self.voice_state = state;
            }
            PeerCommand::MediaAdded(t) => {
                debug!("handle media added {t:?}");

                self.outbound.push(TrackOut {
                    kind: t.kind,
                    state: TrackState::Pending,
                    peer_id: t.peer_id,
                    source_mid: t.source_mid,
                    enabled: false,
                    channel_id: t.channel_id,
                    key: t.key,
                });
            }
            PeerCommand::Kill => self.rtc.disconnect(),
            PeerCommand::GenerateKeyframe {
                mid,
                kind,
                for_peer: _,
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
            PeerCommand::Permissions(p) => {
                self.permissions = p.into();
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
                        layers: vec![],
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
            channel_id: self.voice_state.channel_id,
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
            return;
        };

        if self.voice_state.deafened()
            && track.kind == MediaKind::Audio
            && track.key == TrackKey::User
        {
            return;
        }

        let Some(mid) = track.state.mid() else {
            return;
        };

        let Some(writer) = self.rtc.writer(mid) else {
            return;
        };

        let Some(pt) = writer.match_params(d.params) else {
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
                if let Ok(candidate) = str0m::Candidate::from_sdp_string(&candidate) {
                    self.rtc.add_remote_candidate(candidate);
                }
            }
            SignallingMessage::Want { .. } => todo!(),
            SignallingMessage::Have { .. } => return Err(Error::HaveServerOnly.into()),
            SignallingMessage::Reconnect => panic!("handled by sfu"),
            _ => {}
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
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.signalling.handle_offer(&mut self.rtc, sdp)
        }));

        let answer = match result {
            Ok(Ok(answer)) => answer,
            Ok(Err(e)) => {
                error!("Error handling offer: {}", e);
                return Err(e);
            }
            Err(_e) => {
                error!("Panic handling offer. Disconnecting peer.");
                self.rtc.disconnect();
                return Ok(());
            }
        };

        // renegotiate outbound tracks
        for track in &mut self.outbound {
            if let TrackState::Negotiating(_) = track.state {
                track.state = TrackState::Pending;
            }
        }

        let inbound_old = std::mem::take(&mut self.inbound);
        for track in tracks {
            let mid = str0m::media::Mid::from(&*track.mid);
            let state = inbound_old
                .get(&mid)
                .map(|t| t.state)
                .unwrap_or(TrackState::Negotiating(mid));
            self.inbound.insert(
                mid,
                TrackIn {
                    kind: track.kind.into(),
                    state,
                    channel_id: self.voice_state.channel_id,
                    key: track.key,
                },
            );
        }

        let sdp_str = answer.to_sdp_string();
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
            return Ok(());
        };

        if !matches!(track.state, TrackState::Open(_)) {
            return Ok(());
        };

        if self.voice_state.muted() && track.kind == MediaKind::Audio && track.key == TrackKey::User
        {
            return Ok(());
        }

        if !self.permissions.speak && track.kind == MediaKind::Audio && track.key == TrackKey::User
        {
            return Ok(());
        }

        if !self.permissions.video
            && (track.kind == MediaKind::Video || track.key == TrackKey::Screen)
        {
            return Ok(());
        }

        if !self.voice_state.self_video
            && track.key == TrackKey::User
            && track.kind == MediaKind::Video
        {
            return Ok(());
        }

        if self.voice_state.screenshare.is_none()
            && track.key == TrackKey::Screen
            && track.kind == MediaKind::Video
        {
            return Ok(());
        }

        let payload = MediaData {
            mid: data.mid,
            peer_id: self.user_id,
            network_time: data.network_time,
            time: data.time,
            data: data.data.into(),
            params: data.params,
        };

        // Data Plane: broadcast directly to all other peers
        for (user_id, (_, media_tx)) in &self.routing_table {
            if let Err(e) = media_tx.try_send(PeerMedia::MediaData(payload.clone())) {
                trace!("failed to send media to peer {user_id}: {e}");
            }
        }

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
                    channel_id: track.channel_id,
                }));

                tracks_metadata.push(TrackMetadataServer {
                    source_mid: mid,
                    kind: track.kind.into(),
                    key: track.key.clone(),
                });
            }
        }

        if !tracks_metadata.is_empty() {
            self.tracks_metadata.append(&mut tracks_metadata);

            events.push(PeerEvent::Have {
                tracks: self.tracks_metadata.clone(),
            });

            events.push(PeerEvent::Signalling(SignallingMessage::Have {
                user_id: self.user_id,
                channel_id: self.voice_state.channel_id,
                tracks: self
                    .tracks_metadata
                    .iter()
                    .map(|t| TrackMetadata {
                        mid: TrackId(t.source_mid.to_string()),
                        kind: t.kind,
                        key: t.key.clone(),
                        layers: vec![],
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
