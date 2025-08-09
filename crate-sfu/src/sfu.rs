use crate::{
    config::Config, peer::Peer, PeerCommand, PeerEvent, PeerEventEnvelope, SfuCommand, SfuEvent,
    SfuTrack, SignallingMessage,
};
use anyhow::Result;
use common::v1::types::{util::Time, voice::VoiceState, UserId};
use dashmap::DashMap;
use std::fmt::Debug;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, trace, warn};

pub struct Sfu {
    peers: DashMap<UserId, UnboundedSender<PeerCommand>>,
    voice_states: DashMap<UserId, VoiceState>,
    tracks: Vec<SfuTrack>,
    config: Config,
}

impl Debug for Sfu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sfu")
            .field("peers", &self.peers)
            .field("voice_states", &self.voice_states)
            .field("tracks", &self.tracks)
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
        }
    }

    pub fn spawn(self) -> UnboundedSender<SfuCommand> {
        let (send, recv) = mpsc::unbounded_channel();
        tokio::spawn(self.run(recv));
        send
    }

    async fn run(mut self, mut a: UnboundedReceiver<SfuCommand>) -> Result<()> {
        let (peer_send, mut peer_events) = tokio::sync::mpsc::unbounded_channel();
        loop {
            tokio::select! {
                Some(req) = a.recv() => {
                    if let Err(err) = self.handle_command(req, peer_send.clone()).await {
                        error!("error handling peer command: {err}");
                    }
                }
                Some(envelope) = peer_events.recv() => {
                    if let Err(err) = self.handle_event(envelope).await {
                        error!("error handling peer event: {err}");
                    }
                }
            }
        }
    }

    async fn handle_command(
        &self,
        req: SfuCommand,
        peer_send: UnboundedSender<PeerEventEnvelope>,
    ) -> Result<()> {
        trace!("new rpc message {req:?}");

        let user_id = req.user_id.unwrap();

        match &req.inner {
            SignallingMessage::VoiceState { state } => {
                // user disconnected
                let Some(state) = state else {
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

                // user connected or moved
                // let state = VoiceState {
                //     user_id,
                //     thread_id: state.thread_id,
                //     joined_at: Time::now_utc(),
                // };
                // let old = self.voice_states.insert(user_id, state.clone());
                // debug!("got voice state {state:?}");
                let new_state = VoiceState {
                    user_id,
                    thread_id: state.thread_id,
                    joined_at: Time::now_utc(),
                };
                let old = self.voice_states.insert(user_id, new_state.clone());
                debug!("got voice state {new_state:?}");

                // broadcast all tracks in a thread to the user
                let peer = self
                    .ensure_peer(user_id, peer_send.clone(), &new_state)
                    .await?;
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

                    if let Err(e) = peer.send(PeerCommand::MediaAdded(track.clone())) {
                        warn!("failed to send MediaAdded to peer {}: {}", user_id, e);
                    }
                }

                // tell everyone about the voice state update
                self.emit(SfuEvent::VoiceState {
                    user_id,
                    state: Some(new_state),
                    old,
                })
                .await?;
            }
            _ => {
                let Some(voice_state) = self.voice_states.get(&user_id) else {
                    warn!("no voice state for {user_id}");
                    return Ok(());
                };

                let peer = self
                    .ensure_peer(user_id, peer_send.clone(), &voice_state)
                    .await?;
                peer.send(PeerCommand::Signalling(req.inner))?;
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

                    a.value().send(PeerCommand::MediaData(m.clone()))?;
                }
            }

            PeerEvent::Dead => {
                debug!("peerevent::dead");
                self.peers.remove(&user_id);
                self.tracks.retain(|a| a.peer_id != user_id);
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
                let peer_sender = Peer::spawn(peer_send, user_id, voice_state.clone()).await?;
                entry.insert(peer_sender.clone());
                Ok(peer_sender)
            }
        }
    }

    async fn emit(&self, event: SfuEvent) -> Result<()> {
        reqwest::Client::new()
            .post(format!("{}/api/v1/internal/rpc", self.config.api_url))
            .header("authorization", format!("Server {}", self.config.token))
            .json(&event)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
