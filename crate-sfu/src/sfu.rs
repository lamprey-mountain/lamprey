use crate::{
    peer::Peer, PeerCommand, PeerEvent, PeerEventEnvelope, SfuCommand, SfuEvent, SfuTrack,
    SignallingCommand,
};
use anyhow::Result;
use common::v1::types::{util::Time, voice::VoiceState, UserId};
use dashmap::DashMap;
use str0m::media::Mid;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, trace, warn};

#[derive(Debug, Default)]
pub struct Sfu {
    peers: DashMap<UserId, UnboundedSender<PeerCommand>>,
    voice_states: DashMap<UserId, VoiceState>,
    tracks: Vec<SfuTrack>,
    // config: Config,
}

// #[derive(Debug)]
// pub struct Config {
//     // TODO: use secret scrubbing/zeroizing string here
//     token: String,
// }

impl Sfu {
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
            SignallingCommand::VoiceState { state } => {
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

                let state = VoiceState {
                    user_id,
                    thread_id: state.thread_id,
                    joined_at: Time::now_utc(),
                };
                let old = self.voice_states.insert(user_id, state.clone());
                debug!("got voice state {state:?}");
                self.emit(SfuEvent::VoiceState {
                    user_id,
                    state: Some(state),
                    old,
                })
                .await?;
            }
            SignallingCommand::Publish { mid, key } => {
                let mid = Mid::from(mid.as_str());
                for a in &self.peers {
                    a.value().send(PeerCommand::RemotePublish {
                        user_id,
                        mid,
                        key: key.to_owned(),
                    })?;
                }
            }
            _ => {}
        }

        let ctl = match self.peers.entry(user_id) {
            dashmap::Entry::Occupied(occupied_entry) => occupied_entry.into_ref(),
            dashmap::Entry::Vacant(vacant_entry) => {
                let peer = Peer::spawn(peer_send.clone(), user_id).await?;
                vacant_entry.insert(peer)
            }
        };

        ctl.send(PeerCommand::Signalling(req.inner))?;

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
                debug!("peer event payload {event:?}");
                let Some(my_state) = self.voice_states.get(&user_id) else {
                    warn!("user has no voice state");
                    return Ok(());
                };
                for a in &self.peers {
                    if a.key() == &user_id {
                        continue;
                    }

                    let Some(state) = self.voice_states.get(a.key()) else {
                        continue;
                    };

                    if state.thread_id != my_state.thread_id {
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
                        continue;
                    }

                    let Some(state) = self.voice_states.get(a.key()) else {
                        continue;
                    };

                    if state.thread_id != my_state.thread_id {
                        continue;
                    }

                    a.value().send(PeerCommand::MediaData(m.clone()))?;
                }
            }

            PeerEvent::Dead => {
                self.peers.remove(&user_id);
                self.tracks.retain(|a| a.peer_id != user_id);
            }

            PeerEvent::Init => {
                let Some(my_state) = self.voice_states.get(&user_id) else {
                    warn!("user has no voice state");
                    return Ok(());
                };

                let peer = self.peers.get(&user_id).unwrap();

                for m in &self.tracks {
                    if m.peer_id == user_id {
                        continue;
                    }

                    let Some(state) = self.voice_states.get(&m.peer_id) else {
                        continue;
                    };

                    if state.thread_id != my_state.thread_id {
                        continue;
                    }

                    peer.send(PeerCommand::MediaAdded(m.clone()))?;
                    if let Some(key) = m.key.as_ref() {
                        peer.send(PeerCommand::RemotePublish {
                            user_id,
                            mid: m.mid,
                            key: key.to_owned(),
                        })?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn emit(&self, event: SfuEvent) -> Result<()> {
        reqwest::Client::new()
            .post("http://localhost:4000/api/v1/internal/rpc")
            .header("authorization", "Server verysecrettoken")
            .json(&event)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
