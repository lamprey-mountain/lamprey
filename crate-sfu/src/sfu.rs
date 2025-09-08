use crate::{
    config::Config, peer::Peer, PeerCommand, PeerEvent, PeerEventEnvelope, SfuTrack,
    SignallingMessage, TrackMetadataServer,
};
use anyhow::Result;
use common::v1::types::{
    voice::{SfuCommand, SfuEvent, VoiceState},
    UserId,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use std::fmt::Debug;
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};
use tracing::{debug, error, info, trace, warn};

pub struct Sfu {
    peers: DashMap<UserId, UnboundedSender<PeerCommand>>,
    voice_states: DashMap<UserId, VoiceState>,
    tracks: Vec<SfuTrack>,
    track_metadata: DashMap<UserId, Vec<TrackMetadataServer>>,
    config: Config,
    backend_tx: Option<UnboundedSender<SfuEvent>>,
}

impl Debug for Sfu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sfu")
            .field("peers", &self.peers)
            .field("voice_states", &self.voice_states)
            .field("tracks", &self.tracks)
            .field("backend_tx", &self.backend_tx.is_some())
            .finish()
    }
}

impl Sfu {
    pub fn new(config: Config) -> Self {
        Self {
            peers: DashMap::new(),
            voice_states: DashMap::new(),
            tracks: Vec::new(),
            config,
            backend_tx: None,
            track_metadata: DashMap::new(),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let (peer_send, mut peer_events) = tokio::sync::mpsc::unbounded_channel();
        let (backend_tx, mut backend_rx) = mpsc::unbounded_channel();
        self.backend_tx = Some(backend_tx);

        loop {
            // Reconnection loop
            let url_str = format!("{}/api/v1/internal/rpc", self.config.api_url)
                .replace("http", "ws")
                .replace("https", "wss");

            let mut request = match url_str.into_client_request() {
                Ok(req) => req,
                Err(e) => {
                    error!("Failed to create client request: {}. Retrying in 5s.", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
            request.headers_mut().insert(
                "Authorization",
                format!("Server {}", self.config.token).try_into().unwrap(),
            );

            info!("Connecting to backend websocket...");
            let ws_stream = match connect_async(request).await {
                Ok((stream, _)) => {
                    info!("Connected to backend websocket");
                    stream
                }
                Err(e) => {
                    warn!("Failed to connect to backend: {}. Retrying in 5s.", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let (mut ws_tx, mut ws_rx) = ws_stream.split();

            'inner: loop {
                tokio::select! {
                    Some(envelope) = peer_events.recv() => {
                        if let Err(err) = self.handle_event(envelope).await {
                            error!("error handling peer event: {err}");
                        }
                    }
                    Some(event) = backend_rx.recv() => {
                        let json = match serde_json::to_string(&event) {
                            Ok(j) => j,
                            Err(e) => {
                                error!("Failed to serialize event: {}", e);
                                continue;
                            }
                        };
                        if let Err(e) = ws_tx.send(Message::text(json)).await {
                            error!("Failed to send message to backend: {}", e);
                            break 'inner;
                        }
                    }
                    Some(msg) = ws_rx.next() => {
                        match msg {
                            Ok(Message::Text(t)) => {
                                let command: SfuCommand = serde_json::from_str(&t).unwrap();
                                if let Err(err) = self.handle_command(command, peer_send.clone()).await {
                                    error!("error handling peer command: {err}");
                                }
                            }
                            Ok(Message::Close(_)) => {
                                info!("Backend websocket closed");
                                break 'inner;
                            }
                            Err(e) => {
                                error!("Error receiving from backend: {}", e);
                                break 'inner;
                            }
                            _ => {}
                        }
                    }
                }
            }
            warn!("Disconnected from backend. Reconnecting in 5 seconds...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn handle_command(
        &self,
        cmd: SfuCommand,
        peer_send: UnboundedSender<PeerEventEnvelope>,
    ) -> Result<()> {
        trace!("new rpc message {cmd:?}");

        match cmd {
            SfuCommand::Signalling { user_id, inner } => {
                self.handle_signalling(user_id, inner, peer_send).await?
            }
            SfuCommand::VoiceState {
                user_id,
                thread_id: _,
                state,
            } => {
                let Some(state) = state else {
                    // user disconnected
                    let old = self.voice_states.remove(&user_id).map(|s| s.1);
                    if let Some((_, peer)) = self.peers.remove(&user_id) {
                        peer.send(PeerCommand::Kill)?
                    };
                    self.emit(SfuEvent::VoiceState {
                        user_id,
                        state: None,
                        old,
                    })
                    .await?;
                    debug!("remove voice state");
                    return Ok(());
                };

                debug!("got voice state {state:?}");
                let old = self.voice_states.insert(user_id, state.clone());

                let peer = self.ensure_peer(user_id, peer_send.clone(), &state).await?;

                // broadcast all tracks in a thread to the user
                for track in &self.tracks {
                    if track.peer_id == user_id {
                        continue;
                    }

                    let Some(other) = self.voice_states.get(&track.peer_id) else {
                        warn!("dead track not cleaned up for peer {}", track.peer_id);
                        continue;
                    };

                    if state.thread_id != other.thread_id {
                        continue;
                    }

                    debug!("sending track {track:?}");
                    if let Err(e) = peer.send(PeerCommand::MediaAdded(track.clone())) {
                        warn!("failed to send MediaAdded to peer {}: {}", user_id, e);
                    }
                }

                // also broadcast all the track metadata as well
                for meta in &self.track_metadata {
                    let peer_id = meta.key();

                    if *peer_id == user_id {
                        continue;
                    }

                    let Some(other) = self.voice_states.get(&peer_id) else {
                        warn!("dead track not cleaned up for peer {}", peer_id);
                        continue;
                    };

                    if state.thread_id != other.thread_id {
                        continue;
                    }

                    debug!("sending track_metadata {} {:?}", peer_id, meta.value());
                    if let Err(e) = peer.send(PeerCommand::Have {
                        user_id: *peer_id,
                        tracks: meta.value().clone(),
                    }) {
                        warn!("failed to send Have to peer {}: {}", user_id, e);
                    }
                }

                // tell everyone about the voice state update
                self.emit(SfuEvent::VoiceState {
                    user_id,
                    state: Some(state),
                    old,
                })
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_signalling(
        &self,
        user_id: UserId,
        msg: SignallingMessage,
        peer_send: UnboundedSender<PeerEventEnvelope>,
    ) -> Result<()> {
        match &msg {
            SignallingMessage::VoiceState { .. } => {
                warn!("raw signalling messages should not be sent here");
            }
            _ => {
                let Some(voice_state) = self.voice_states.get(&user_id) else {
                    warn!("no voice state for {user_id}");
                    return Ok(());
                };

                let peer = self
                    .ensure_peer(user_id, peer_send.clone(), &voice_state)
                    .await?;
                peer.send(PeerCommand::Signalling(msg))?;
            }
        }

        Ok(())
    }

    async fn handle_event(&mut self, envelope: PeerEventEnvelope) -> Result<()> {
        let user_id = envelope.user_id;
        let event = envelope.payload;
        match event {
            PeerEvent::Signalling(payload) => {
                debug!("signalling event {payload:?}");
                self.emit(SfuEvent::VoiceDispatch { user_id, payload })
                    .await?;
            }
            PeerEvent::MediaAdded(ref m) => {
                debug!("media added event {event:?}");
                let Some(my_state) = self.voice_states.get(&user_id) else {
                    warn!("user has no voice state");
                    return Ok(());
                };
                if self
                    .tracks
                    .iter()
                    .any(|t| t.source_mid == m.source_mid && t.peer_id == user_id)
                {
                    debug!("skipping this track, we already have it");
                    return Ok(());
                }
                for a in &self.peers {
                    if a.key() == &user_id {
                        debug!("drop: no echo");
                        continue;
                    }

                    let Some(state) = self.voice_states.get(a.key()) else {
                        debug!("drop: no voice state");
                        continue;
                    };

                    if state.thread_id != my_state.thread_id {
                        debug!("drop: no thread id");
                        continue;
                    }

                    a.value().send(PeerCommand::MediaAdded(m.clone()))?;
                }
                self.tracks.push(m.clone());
            }
            PeerEvent::MediaData(m) => {
                // debug!("media data event");
                let Some(my_state) = self.voice_states.get(&user_id) else {
                    warn!("user has no voice state");
                    return Ok(());
                };
                for a in &self.peers {
                    if a.key() == &user_id {
                        // debug!("skip own user");
                        continue;
                    }

                    let Some(state) = self.voice_states.get(a.key()) else {
                        debug!("missing voice state");
                        continue;
                    };

                    if state.thread_id != my_state.thread_id {
                        // debug!("wrong thread id");
                        continue;
                    }

                    a.value().send(PeerCommand::MediaData(m.clone()))?;
                }
            }
            PeerEvent::Dead => {
                debug!("peerevent::dead");
                self.peers.remove(&user_id);
                self.track_metadata.remove(&user_id);
                self.tracks.retain(|a| a.peer_id != user_id);
            }
            PeerEvent::NeedsKeyframe {
                source_mid,
                source_peer,
                for_peer,
                kind,
                rid,
            } => {
                debug!("needs keyframe event {event:?}");
                let Some(peer) = self.peers.get(&source_peer) else {
                    warn!("peer not found");
                    return Ok(());
                };

                peer.send(PeerCommand::GenerateKeyframe {
                    mid: source_mid,
                    kind,
                    for_peer,
                    rid,
                })?;
            }
            PeerEvent::Have { tracks } => {
                debug!("have event {tracks:?}");
                let Some(my_state) = self.voice_states.get(&user_id) else {
                    warn!("user has no voice state");
                    return Ok(());
                };
                for a in &self.peers {
                    if a.key() == &user_id {
                        debug!("skip own user");
                        continue;
                    }

                    let Some(state) = self.voice_states.get(a.key()) else {
                        debug!("missing voice state");
                        continue;
                    };

                    if state.thread_id != my_state.thread_id {
                        debug!("wrong thread id");
                        continue;
                    }

                    a.value().send(PeerCommand::Have {
                        user_id,
                        tracks: tracks.clone(),
                    })?;
                }
                self.track_metadata.insert(user_id, tracks);
            }
            PeerEvent::WantHave { user_ids } => {
                let (Some(state), Some(peer)) =
                    (self.voice_states.get(&user_id), self.peers.get(&user_id))
                else {
                    warn!("received peer event from dead peer?");
                    return Ok(());
                };

                for peer_id in user_ids {
                    if peer_id == user_id {
                        continue;
                    }

                    let Some(other) = self.voice_states.get(&peer_id) else {
                        warn!("dead track not cleaned up for peer {}", peer_id);
                        continue;
                    };

                    if state.thread_id != other.thread_id {
                        continue;
                    }

                    let Some(meta) = self.track_metadata.get(&peer_id) else {
                        warn!("missing metadata for peer {}", peer_id);
                        continue;
                    };

                    debug!("sending requested track_metadata {} {:?}", peer_id, meta);
                    if let Err(e) = peer.send(PeerCommand::Have {
                        user_id: peer_id,
                        tracks: meta.to_owned(),
                    }) {
                        warn!("failed to send Have to peer {}: {}", user_id, e);
                    }
                }
            }
            PeerEvent::Speaking(speaking) => {
                let Some(my_state) = self.voice_states.get(&user_id) else {
                    warn!("user has no voice state");
                    return Ok(());
                };

                for a in &self.peers {
                    if a.key() == &user_id {
                        debug!("skip own user");
                        continue;
                    }

                    let Some(state) = self.voice_states.get(a.key()) else {
                        debug!("missing voice state");
                        continue;
                    };

                    if state.thread_id != my_state.thread_id {
                        debug!("wrong thread id");
                        continue;
                    }

                    a.value().send(PeerCommand::Speaking(speaking.clone()))?;
                }
            }
        }

        Ok(())
    }

    async fn ensure_peer(
        &self,
        user_id: UserId,
        peer_send: UnboundedSender<PeerEventEnvelope>,
        voice_state: &VoiceState,
    ) -> Result<UnboundedSender<PeerCommand>> {
        match self.peers.entry(user_id) {
            dashmap::Entry::Occupied(entry) => Ok(entry.get().clone()),
            dashmap::Entry::Vacant(entry) => {
                let peer_sender =
                    Peer::spawn(&self.config, peer_send, user_id, voice_state.clone()).await?;
                entry.insert(peer_sender.clone());
                Ok(peer_sender)
            }
        }
    }

    async fn emit(&self, event: SfuEvent) -> Result<()> {
        if let Some(tx) = &self.backend_tx {
            tx.send(event).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        Ok(())
    }
}
