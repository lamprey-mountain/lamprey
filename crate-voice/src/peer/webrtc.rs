use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Instant};

use async_trait::async_trait;
use bytes::Bytes;
use common::v1::types::{
    voice::{
        internal::{MediaData, SfuPermissions},
        messages::{PeerEvent, SignallingCommand, SignallingEvent},
        Mid, SessionDescription, Speaking, SpeakingWithUserId, TrackKey, TrackMetadata,
        TrackMetadataWithUserId, VoiceState,
    },
    ChannelId, UserId,
};
use str0m::{
    media::{Direction, MediaKind, Mid as SMid},
    Candidate, Event, Input, Output, Rtc,
};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep_until;
use tokio::{net::UdpSocket, sync::broadcast};
use tracing::{debug, error, warn};

use crate::{
    peer::{Command, CommandFull, Peer},
    signalling::Signalling,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackState {
    Pending,
    Negotiating(SMid),
    Open(SMid),
}

impl TrackState {
    pub fn mid(&self) -> Option<SMid> {
        match self {
            TrackState::Pending => None,
            TrackState::Negotiating(mid) => Some(*mid),
            TrackState::Open(mid) => Some(*mid),
        }
    }
}

#[derive(Debug)]
pub struct TrackIn {
    pub kind: MediaKind,
    pub state: TrackState,
    pub channel_id: ChannelId,
    pub key: TrackKey,
}

#[derive(Debug)]
pub struct TrackOut {
    pub kind: MediaKind,
    pub state: TrackState,
    pub user_id: UserId,
    pub source_mid: Mid,
    pub enabled: bool,
    pub channel_id: ChannelId,
    pub key: TrackKey,
}

/// a handle to a webrtc peer connection
#[derive(Debug, Clone)]
pub struct PeerWebrtc {
    user_id: UserId,
    command_tx: mpsc::UnboundedSender<CommandFull>,
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<PeerEvent>>>,
}

/// the actor responsible for the webrtc connection lifecycle
pub struct PeerWebrtcInner {
    user_id: UserId,
    rtc: Rtc,
    socket_v4: Arc<UdpSocket>,
    socket_v6: Arc<UdpSocket>,
    voice_state: VoiceState,
    permissions: SfuPermissions,
    signalling: Signalling,
    command_rx: mpsc::UnboundedReceiver<CommandFull>,
    broadcast_rx: broadcast::Receiver<Arc<CommandFull>>,
    event_tx: mpsc::UnboundedSender<PeerEvent>,

    inbound: HashMap<SMid, TrackIn>,
    outbound: Vec<TrackOut>,

    speaking_chan: Option<str0m::channel::ChannelId>,
    last_ufrag: Option<String>,
}

impl PeerWebrtc {
    pub fn spawn(
        user_id: UserId,
        voice_state: VoiceState,
        permissions: SfuPermissions,
        socket_v4: Arc<UdpSocket>,
        socket_v6: Arc<UdpSocket>,
        broadcast_rx: broadcast::Receiver<Arc<CommandFull>>,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let mut rtc_config = str0m::RtcConfig::new().set_ice_lite(true).build();

        if let Ok(addr) = socket_v4.local_addr() {
            if let Ok(c) = Candidate::host(addr, "udp") {
                rtc_config.add_local_candidate(c);
            }
        }
        if let Ok(addr) = socket_v6.local_addr() {
            if let Ok(c) = Candidate::host(addr, "udp") {
                rtc_config.add_local_candidate(c);
            }
        }

        let inner = PeerWebrtcInner {
            user_id,
            rtc: rtc_config,
            socket_v4,
            socket_v6,
            voice_state,
            permissions,
            signalling: Signalling::new(),
            command_rx,
            broadcast_rx,
            event_tx,
            inbound: HashMap::new(),
            outbound: vec![],
            speaking_chan: None,
            last_ufrag: None,
        };

        tokio::spawn(async move {
            if let Err(e) = inner.run().await {
                error!("PeerWebrtc loop failed for user {}: {:?}", user_id, e);
            }
        });

        Self {
            user_id,
            command_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
        }
    }

    pub fn handle_network_packet(&self, source: SocketAddr, data: Bytes) {
        _ = self
            .command_tx
            .send(CommandFull::NetworkPacket(source, data));
    }
}

// TODO: don't send any audio if `self.voice_state.deafened()`
// TODO: don't receive any audio if `self.voice_state.muted()`
// TODO: disallow sending user audio if permissions.speak is denied
// TODO: disallow sending user video if permissions.video is denied
// TODO: disallow starting screenshare, camera, etc if permissions.video is denied

// logic summary
// if permissions.video is denied, ONLY allow a track of kind=audio, key=user
// if permissions.speak is denied, DISALLOW a track of kind=audio, key=user (other keys are fine, including screenshare. screenshare can have audio.)
// if both are denied, don't allow creating any tracks at all
// if both are allowed, allow creating any tracks
impl PeerWebrtcInner {
    async fn run(mut self) -> Result<(), anyhow::Error> {
        loop {
            if let Err(e) = self.negotiate_if_needed() {
                warn!("Negotiation failed: {:?}", e);
            }

            let ufrag = self
                .rtc
                .direct_api()
                .local_ice_credentials()
                .ufrag
                .to_string();
            if self.last_ufrag.as_ref() != Some(&ufrag) {
                self.last_ufrag = Some(ufrag.clone());
                self.emit(PeerEvent::IceUfrag(ufrag));
            }

            let timeout = match self.rtc.poll_output()? {
                Output::Timeout(t) => t,
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
                Output::Event(e) => {
                    self.handle_rtc_event(e);
                    continue;
                }
            };

            tokio::select! {
                Some(cmd) = self.command_rx.recv() => {
                    self.handle_command_full(cmd);
                }
                res = self.broadcast_rx.recv() => {
                    match res {
                        Ok(cmd) => self.handle_broadcast(cmd),
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // TODO: handle lag
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // channel closed, probably shut down
                            break;
                        }
                    }
                }
                _ = sleep_until(timeout.into()) => {
                    if let Err(e) = self.rtc.handle_input(Input::Timeout(Instant::now())) {
                        warn!("timeout disconnect: {:?}", e);
                        break;
                    }
                }
            }

            if !self.rtc.is_alive() {
                debug!("RTC is no longer alive, exiting run loop.");
                break;
            }
        }
        Ok(())
    }

    fn negotiate_if_needed(&mut self) -> Result<(), anyhow::Error> {
        let mut change = self.rtc.sdp_api();
        for track in &mut self.outbound {
            if track.state == TrackState::Pending {
                let mid = change.add_media(track.kind, Direction::SendOnly, None, None, None);
                track.state = TrackState::Negotiating(mid);
            }
        }

        if let Some(offer) = self.signalling.negotiate_if_needed(change)? {
            self.emit(PeerEvent::Signalling(SignallingEvent::Offer {
                sdp: SessionDescription(offer.to_sdp_string()),
                tracks: vec![], // TODO: populate
            }));
        }

        Ok(())
    }

    fn handle_rtc_event(&mut self, event: Event) {
        match event {
            Event::Connected => {
                debug!("connected!");
                self.emit(PeerEvent::Connected);
            }
            Event::MediaAdded(m) => {
                debug!("media added {:?}", m);
                if let Some(track) = self.inbound.get_mut(&m.mid) {
                    track.state = TrackState::Open(m.mid);
                }

                if let Some(track) = self.inbound.get(&m.mid) {
                    self.emit(PeerEvent::MediaAdded(TrackMetadataWithUserId {
                        inner: TrackMetadata {
                            kind: track.kind.into(),
                            key: track.key.clone(),
                            mid: m.mid.into(),
                            layers: vec![], // TODO: populate
                        },
                        user_id: self.user_id,
                    }));
                } else {
                    panic!("track disappeared")
                }
            }
            Event::MediaChanged(_) => {
                // TODO: handle?
                // currently this only is emitted when direction is changed, which i probably dont need to handle
            }
            Event::MediaData(m) => {
                let payload = MediaData {
                    mid: m.mid.into(),
                    user_id: self.user_id,
                    network_time: m.network_time,
                    time: m.time,
                    data: m.data.into(),

                    // FIXME: make sure pt matches `str0m::media::Pt` correctly
                    pt: m.pt,
                };
                self.emit(PeerEvent::MediaData(payload));
            }
            Event::KeyframeRequest(r) => {
                debug!("keyframe request {:?}", r);
                let track = self.outbound.iter().find(|t| t.state.mid() == Some(r.mid));
                if let Some(track) = track {
                    self.emit(PeerEvent::KeyframeRequest {
                        source_mid: track.source_mid,
                        user_id: track.user_id,
                        kind: r.kind.into(),
                        rid: r.rid.map(|r| r.into()),
                    });
                } else {
                    warn!("track not found for keyframe request");
                }
            }
            Event::ChannelOpen(chan_id, label) => {
                if label == "speaking" {
                    self.speaking_chan = Some(chan_id);
                }
            }
            Event::ChannelData(data) => {
                if self.speaking_chan == Some(data.id) {
                    if let Ok(speaking) = Speaking::from_bytes(&data.data) {
                        self.emit(PeerEvent::Speaking(SpeakingWithUserId {
                            user_id: self.user_id,
                            flags: speaking.flags,
                            source_mid: speaking.mid,
                        }));
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_broadcast(&mut self, command: Arc<CommandFull>) {
        match &*command {
            CommandFull::Inner(command) => match command {
                Command::MediaAdded(m) => {
                    self.outbound.push(TrackOut {
                        kind: m.inner.kind.into(),
                        state: TrackState::Pending,
                        user_id: m.user_id,
                        source_mid: m.inner.mid,
                        enabled: false,
                        channel_id: self.voice_state.channel_id,
                        key: m.inner.key.clone(),
                    });
                }
                Command::GenerateKeyframe {
                    mid,
                    rid,
                    kind,
                    user_id,
                } => {
                    if let Some(mut w) = self.rtc.writer((*mid).into()) {
                        let r = rid.map(|r| r.into());
                        let _ = w.request_keyframe(r, (*kind).into());
                    }
                }
                _ => {}
            },
            CommandFull::MediaData(m) => {
                if m.user_id == self.user_id {
                    return;
                }

                let Some(track) = self
                    .outbound
                    .iter()
                    .find(|t| t.source_mid == m.mid && t.user_id == m.user_id)
                else {
                    return;
                };
                let Some(mid) = track.state.mid() else { return };
                let Some(mut writer) = self.rtc.writer(mid) else {
                    return;
                };

                // for now, just write the data
                // FIXME: handle pt mapping properly (based on sdp)
                let _ = writer.write(m.pt, m.network_time, m.time, m.data.as_ref());
            }
            CommandFull::Speaking(s) => {
                if s.user_id == self.user_id {
                    return;
                }

                if let Some(chan) = self.speaking_chan {
                    if let Some(mut c) = self.rtc.channel(chan) {
                        let speaking = Speaking {
                            mid: s.source_mid,
                            flags: s.flags,
                        };
                        let bytes = speaking.to_bytes();
                        let _ = c.write(true, &bytes);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_command_full(&mut self, command: CommandFull) {
        match command {
            CommandFull::Inner(command) => self.handle_command(command),
            CommandFull::MediaData(m) => {
                let Some(track) = self
                    .outbound
                    .iter()
                    .find(|t| t.source_mid == m.mid && t.user_id == m.user_id)
                else {
                    return;
                };
                let Some(mid) = track.state.mid() else { return };
                let Some(mut writer) = self.rtc.writer(mid) else {
                    return;
                };

                // writer.match_params(params)

                // for now, just write the data
                // FIXME: handle pt mapping properly (based on sdp)
                let _ = writer.write(m.pt, m.network_time, m.time, m.data.as_ref());
            }
            CommandFull::Speaking(s) => {
                if let Some(chan) = self.speaking_chan {
                    if let Some(mut c) = self.rtc.channel(chan) {
                        let speaking = Speaking {
                            mid: s.source_mid,
                            flags: s.flags,
                        };
                        let bytes = speaking.to_bytes();
                        let _ = c.write(true, &bytes);
                    }
                }
            }
            CommandFull::NetworkPacket(source, data) => {
                let local_addr = if source.is_ipv4() {
                    self.socket_v4.local_addr()
                } else {
                    self.socket_v6.local_addr()
                };

                let local_addr = match local_addr {
                    Ok(a) => a,
                    Err(e) => {
                        error!("Failed to get local address: {:?}", e);
                        return;
                    }
                };

                let contents = match data.as_ref().try_into() {
                    Ok(c) => c,
                    Err(_) => return,
                };

                let input = str0m::Input::Receive(
                    Instant::now(),
                    str0m::net::Receive {
                        proto: str0m::net::Protocol::Udp,
                        source,
                        destination: local_addr,
                        contents,
                    },
                );

                if let Err(e) = self.rtc.handle_input(input) {
                    warn!("failed to handle input: {:?}", e);
                }
            }
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::Signalling(cmd) => match cmd {
                SignallingCommand::Answer { sdp } => {
                    if let Err(e) = self.signalling.handle_answer(&mut self.rtc, sdp) {
                        warn!("Failed to handle answer: {:?}", e);
                    } else {
                        for track in &mut self.outbound {
                            if let TrackState::Negotiating(m) = track.state {
                                track.state = TrackState::Open(m);
                            }
                        }
                    }
                }
                SignallingCommand::Offer { sdp, tracks } => {
                    match self.signalling.handle_offer(&mut self.rtc, sdp) {
                        Ok(answer) => {
                            for track in &mut self.outbound {
                                if let TrackState::Negotiating(_) = track.state {
                                    track.state = TrackState::Pending;
                                }
                            }

                            for track in tracks {
                                let mid: SMid = track.mid.into();
                                self.inbound.insert(
                                    mid,
                                    TrackIn {
                                        kind: track.kind.into(),
                                        state: TrackState::Negotiating(mid),
                                        channel_id: self.voice_state.channel_id,
                                        key: track.key,
                                    },
                                );
                            }

                            self.emit(PeerEvent::Signalling(SignallingEvent::Answer {
                                sdp: SessionDescription(answer.to_sdp_string()),
                            }));
                        }
                        Err(e) => warn!("Failed to handle offer: {:?}", e),
                    }
                }
                SignallingCommand::Candidate { candidate } => {
                    if let Ok(c) = str0m::Candidate::from_sdp_string(&candidate) {
                        self.rtc.add_remote_candidate(c);
                    }
                }
                SignallingCommand::Disconnect => {
                    self.rtc.disconnect();
                }
                SignallingCommand::VoiceState { state } => {
                    self.voice_state.apply(state);
                }
                SignallingCommand::Want { subscriptions } => {
                    // TODO: handle subscriptions
                    debug!("Want subscriptions: {:?}", subscriptions);
                }
            },
            Command::GenerateKeyframe {
                mid,
                rid,
                kind,
                user_id: _,
            } => {
                if let Some(mut w) = self.rtc.writer(mid.into()) {
                    let r = rid.map(|r| r.into());
                    let _ = w.request_keyframe(r, kind.into());
                }
            }
            Command::MediaAdded(t) => {
                self.outbound.push(TrackOut {
                    kind: t.inner.kind.into(),
                    state: TrackState::Pending,
                    user_id: self.user_id,
                    source_mid: t.inner.mid,
                    enabled: false,
                    channel_id: self.voice_state.channel_id,
                    key: t.inner.key,
                });
            }
        }
    }

    fn emit(&self, event: PeerEvent) {
        let _ = self.event_tx.send(event);
    }
}

#[async_trait]
impl Peer for PeerWebrtc {
    fn id(&self) -> UserId {
        self.user_id
    }

    fn handle_command(&self, cmd: Command) {
        _ = self.command_tx.send(CommandFull::Inner(cmd));
    }

    fn handle_media_data(&self, media: MediaData) {
        _ = self.command_tx.send(CommandFull::MediaData(media));
    }

    fn handle_speaking(&self, speaking: SpeakingWithUserId) {
        _ = self.command_tx.send(CommandFull::Speaking(speaking));
    }

    async fn poll(&mut self) -> Option<PeerEvent> {
        self.event_rx.lock().await.recv().await
    }
}
