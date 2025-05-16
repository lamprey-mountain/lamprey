use crate::{
    peer::Peer, PeerCommand, PeerEvent, PeerEventEnvelope, SfuCommand, SfuEvent, SfuTrack,
    SignallingCommand, SignallingEvent, TrackState,
};
use anyhow::Result;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json,
};
use common::v1::types::{util::Time, voice::VoiceState, UserId};
use dashmap::DashMap;
use str0m::media::{MediaKind, Mid};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{debug, trace};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Default)]
pub struct Sfu {
    peers: DashMap<UserId, UnboundedSender<PeerCommand>>,
    voice_states: DashMap<UserId, VoiceState>,

    // TODO: cleanup old/disconnected media
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
                Some(req) = a.recv() => self.handle_command(req, peer_send.clone()).await?,
                Some(envelope) = peer_events.recv() => self.handle_event(envelope).await?,
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
        let ctl = match self.peers.entry(user_id) {
            dashmap::Entry::Occupied(occupied_entry) => occupied_entry.into_ref(),
            dashmap::Entry::Vacant(vacant_entry) => {
                let peer = Peer::spawn(peer_send.clone(), user_id).await?;
                for m in &self.tracks {
                    peer.send(PeerCommand::MediaAdded(m.clone()))?;
                }

                vacant_entry.insert(peer)
            }
        };

        match &req.inner {
            SignallingCommand::VoiceStateUpdate { patch } => {
                if let Some(thread_id) = patch.thread_id {
                    self.voice_states.insert(
                        user_id,
                        VoiceState {
                            user_id,
                            thread_id,
                            joined_at: Time::now_utc(),
                        },
                    );
                } else {
                    self.voice_states.remove(&user_id);
                }
                self.emit(SfuEvent::VoiceDispatch {
                    user_id,
                    payload: SignallingEvent::VoiceState {
                        user_id,
                        state: self.voice_states.get(&user_id).map(|s| s.to_owned()),
                    },
                })
                .await?;
                debug!("got voice state update {patch:?}");
            }
            _ => {}
        }

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
                for a in &self.peers {
                    if a.key() != &user_id {
                        a.value().send(PeerCommand::MediaAdded(m.clone()))?;
                    }
                }
                self.tracks.push(m.clone());
            }

            PeerEvent::MediaData(m) => {
                for a in &self.peers {
                    if a.key() != &user_id {
                        a.value().send(PeerCommand::MediaData(m.clone()))?;
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
