use crate::{
    backend::BackendConnection, config::Config, peer::Peer, PeerCommand, PeerEvent,
    PeerEventEnvelope, SignallingMessage, TrackMetadataServer, TrackMetadataSfu,
};
use anyhow::Result;
use common::v1::types::{
    voice::{SfuCommand, SfuEvent, SfuPermissions, SfuThread, VoiceState},
    SfuId, ThreadId, UserId,
};
use dashmap::DashMap;
use std::fmt::Debug;
use tokio::sync::mpsc::{self, UnboundedSender};
use tracing::{debug, error, trace, warn};

#[derive(Debug)]
struct SfuVoiceState {
    state: VoiceState,
    permissions: SfuPermissions,
}

pub struct Sfu {
    peers: DashMap<UserId, UnboundedSender<PeerCommand>>,
    voice_states: DashMap<UserId, SfuVoiceState>,
    tracks: Vec<TrackMetadataSfu>,
    tracks_by_user: DashMap<UserId, Vec<TrackMetadataServer>>,
    // TODO: cleanup unused threads
    threads: DashMap<ThreadId, SfuThread>,
    config: Config,
    backend_tx: UnboundedSender<SfuEvent>,

    /// the uuid assigned to us by backend
    sfu_id: Option<SfuId>,
}

impl Debug for Sfu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sfu")
            .field("peers", &self.peers)
            .field("voice_states", &self.voice_states)
            .field("tracks", &self.tracks)
            .field("backend_tx", &self.backend_tx)
            .finish()
    }
}

impl Sfu {
    pub fn new(config: Config, backend_tx: UnboundedSender<SfuEvent>) -> Self {
        Self {
            peers: DashMap::new(),
            voice_states: DashMap::new(),
            tracks: Vec::new(),
            config,
            backend_tx,
            tracks_by_user: DashMap::new(),
            sfu_id: None,
            threads: DashMap::new(),
        }
    }

    pub async fn run(config: Config) {
        let (peer_send, mut peer_events) = mpsc::unbounded_channel::<PeerEventEnvelope>();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (command_tx, mut command_rx) = mpsc::unbounded_channel();

        let backend = BackendConnection::new(config.clone(), event_rx, command_tx);
        tokio::spawn(backend.spawn());

        let mut sfu = Sfu::new(config, event_tx);

        loop {
            tokio::select! {
                Some(envelope) = peer_events.recv() => {
                    if let Err(err) = sfu.handle_event(envelope.user_id, envelope.payload).await {
                        error!("error handling peer event: {err}");
                    }
                }
                Some(command) = command_rx.recv() => {
                    if let Err(err) = sfu.handle_command(command, peer_send.clone()).await {
                        error!("error handling peer command: {err}");
                    }
                }
            }
        }
    }

    async fn handle_command(
        &mut self,
        cmd: SfuCommand,
        peer_send: UnboundedSender<PeerEventEnvelope>,
    ) -> Result<()> {
        trace!("new rpc message {cmd:?}");

        match cmd {
            SfuCommand::Ready { sfu_id } => {
                self.sfu_id = Some(sfu_id);
            }
            SfuCommand::Signalling { user_id, inner } => {
                self.handle_signalling(user_id, inner, peer_send).await?
            }
            SfuCommand::VoiceState {
                user_id,
                state,
                permissions,
            } => {
                self.handle_voice_state(user_id, state, permissions, peer_send)
                    .await?
            }
            SfuCommand::Thread { thread } => {
                self.threads.insert(thread.id, thread);
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, peer_send))]
    async fn handle_voice_state(
        &self,
        user_id: UserId,
        state: Option<VoiceState>,
        permissions: SfuPermissions,
        peer_send: UnboundedSender<PeerEventEnvelope>,
    ) -> Result<()> {
        let Some(state) = state else {
            // user disconnected
            let old = self.voice_states.remove(&user_id).map(|s| s.1);
            if let Some((_, peer)) = self.peers.remove(&user_id) {
                peer.send(PeerCommand::Kill)?
            };
            self.emit(SfuEvent::VoiceState {
                user_id,
                state: None,
                old: old.map(|o| o.state),
            })
            .await?;
            debug!("remove voice state");
            return Ok(());
        };

        debug!("got voice state {state:?}");
        let old = self.voice_states.insert(
            user_id,
            SfuVoiceState {
                state: state.clone(),
                permissions: permissions.clone(),
            },
        );

        let peer = self
            .ensure_peer(user_id, peer_send.clone(), &state, &permissions)
            .await?;

        // broadcast all tracks in a thread to the user
        for track in &self.tracks {
            if track.peer_id == user_id {
                continue;
            }

            let Some(other) = self.voice_states.get(&track.peer_id) else {
                warn!("dead track not cleaned up for peer {}", track.peer_id);
                continue;
            };

            if state.thread_id != other.state.thread_id {
                continue;
            }

            debug!("sending track {track:?}");
            if let Err(e) = peer.send(PeerCommand::MediaAdded(track.clone())) {
                warn!("failed to send MediaAdded to peer {}: {}", user_id, e);
            }
        }

        // also broadcast all the track metadata as well
        for meta in &self.tracks_by_user {
            let peer_id = meta.key();

            if *peer_id == user_id {
                continue;
            }

            let Some(other) = self.voice_states.get(&peer_id) else {
                warn!("dead track not cleaned up for peer {}", peer_id);
                continue;
            };

            if state.thread_id != other.state.thread_id {
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
            old: old.map(|o| o.state),
        })
        .await?;

        // we're ready for the peer to send us stuff!
        self.emit(SfuEvent::VoiceDispatch {
            user_id,
            payload: SignallingMessage::Ready {
                sfu_id: self
                    .sfu_id
                    .expect("we always receive a Ready before anything else"),
            },
        })
        .await?;

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
            SignallingMessage::Reconnect {} => {
                let Some(voice) = self.voice_states.get(&user_id) else {
                    warn!("no voice state for {user_id}");
                    return Ok(());
                };

                if let Some((_, peer)) = self.peers.remove(&user_id) {
                    peer.send(PeerCommand::Kill)?;
                }

                self.ensure_peer(user_id, peer_send.clone(), &voice.state, &voice.permissions)
                    .await?;
            }
            _ => {
                let Some(voice) = self.voice_states.get(&user_id) else {
                    warn!("no voice state for {user_id}");
                    return Ok(());
                };

                let peer = self
                    .ensure_peer(user_id, peer_send.clone(), &voice.state, &voice.permissions)
                    .await?;
                peer.send(PeerCommand::Signalling(msg))?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, event))]
    async fn handle_event(&mut self, user_id: UserId, event: PeerEvent) -> Result<()> {
        if !matches!(event, PeerEvent::MediaData(_)) {
            debug!("handle event {event:?}");
        }

        match event {
            PeerEvent::Signalling(payload) => {
                self.emit(SfuEvent::VoiceDispatch { user_id, payload })
                    .await?;
            }
            PeerEvent::MediaAdded(m) => {
                if self
                    .tracks
                    .iter()
                    .any(|t| t.source_mid == m.source_mid && t.peer_id == user_id)
                {
                    debug!("skipping this track, we already have it");
                    return Ok(());
                }
                self.broadcast_thread(user_id, PeerCommand::MediaAdded(m.clone()))
                    .await?;
                self.tracks.push(m);
            }
            PeerEvent::MediaData(m) => {
                self.broadcast_thread(user_id, PeerCommand::MediaData(m))
                    .await?;
            }
            PeerEvent::Dead => {
                debug!("peerevent::dead");
                self.peers.remove(&user_id);
                self.tracks_by_user.remove(&user_id);
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
                self.tracks_by_user.insert(user_id, tracks.clone());
                self.broadcast_thread(user_id, PeerCommand::Have { user_id, tracks })
                    .await?;
            }
            PeerEvent::WantHave { user_ids } => {
                self.handle_want_have(user_id, &user_ids).await?;
            }
            PeerEvent::Speaking(speaking) => {
                self.broadcast_thread(user_id, PeerCommand::Speaking(speaking))
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_want_have(&self, user_id: UserId, user_ids: &[UserId]) -> Result<()> {
        let (Some(voice), Some(peer)) = (self.voice_states.get(&user_id), self.peers.get(&user_id))
        else {
            warn!("received peer event from dead peer?");
            return Ok(());
        };

        for peer_id in user_ids {
            let Some(other) = self.voice_states.get(&peer_id) else {
                warn!("dead track not cleaned up for peer {}", peer_id);
                continue;
            };

            if voice.state.thread_id != other.state.thread_id {
                continue;
            }

            let Some(meta) = self.tracks_by_user.get(&peer_id) else {
                warn!("missing metadata for peer {}", peer_id);
                continue;
            };

            debug!("sending requested track_metadata {} {:?}", peer_id, meta);
            if let Err(e) = peer.send(PeerCommand::Have {
                user_id: *peer_id,
                tracks: meta.to_owned(),
            }) {
                warn!("failed to send Have to peer {}: {}", user_id, e);
            }
        }

        Ok(())
    }

    async fn ensure_peer(
        &self,
        user_id: UserId,
        peer_send: UnboundedSender<PeerEventEnvelope>,
        voice_state: &VoiceState,
        permissions: &SfuPermissions,
    ) -> Result<UnboundedSender<PeerCommand>> {
        match self.peers.entry(user_id) {
            dashmap::Entry::Occupied(entry) => Ok(entry.get().clone()),
            dashmap::Entry::Vacant(entry) => {
                let peer_sender = Peer::spawn(
                    &self.config,
                    peer_send,
                    user_id,
                    voice_state.clone(),
                    permissions.clone(),
                )
                .await?;
                entry.insert(peer_sender.clone());
                Ok(peer_sender)
            }
        }
    }

    /// send a command to every peer in a thread
    async fn broadcast_thread(&self, user_id: UserId, command: PeerCommand) -> Result<()> {
        let Some(my_voice) = self.voice_states.get(&user_id) else {
            warn!("user has no voice state");
            return Ok(());
        };

        for peer in &self.peers {
            if peer.key() == &user_id {
                continue;
            }

            let Some(voice) = self.voice_states.get(peer.key()) else {
                debug!("missing voice state");
                continue;
            };

            if voice.state.thread_id != my_voice.state.thread_id {
                continue;
            }

            peer.value().send(command.clone())?;
        }

        Ok(())
    }

    /// emit an event to backend
    async fn emit(&self, event: SfuEvent) -> Result<()> {
        self.backend_tx
            .send(event)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
}
