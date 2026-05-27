use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use common::v1::types::{
    voice::{
        internal::MediaData,
        messages::{
            BackboneDatagram, BackboneDispatch, BackboneDispatchEnvelope, PeerEvent,
            SignallingCommand,
        },
        SpeakingWithUserId,
    },
    SfuId, UserId,
};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::{
    backbone::BackboneComms,
    peer::{Command, CommandFull, Peer},
};

/// a handle to a cascaded peer connection
#[derive(Debug)]
pub struct PeerCascading {
    // id: UserId,
    command_tx: mpsc::UnboundedSender<CommandFull>,
    event_rx: mpsc::UnboundedReceiver<PeerEvent>,
}

/// the actor responsible for the cascade lifecycle
pub struct PeerCascadingInner {
    // id: UserId,
    /// the remote sfu this cascading peer represents
    remote_sfu: SfuId,

    backbone: Arc<BackboneComms>,
    command_rx: mpsc::UnboundedReceiver<CommandFull>,
    event_tx: mpsc::UnboundedSender<PeerEvent>,
}

impl PeerCascading {
    pub fn spawn(remote_sfu: SfuId, backbone: Arc<BackboneComms>) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let inner = PeerCascadingInner {
            // id,
            remote_sfu,
            backbone,
            command_rx,
            event_tx,
        };

        tokio::spawn(async move {
            inner.run().await;
        });

        Self {
            command_tx,
            event_rx,
        }
    }
}

impl PeerCascadingInner {
    async fn run(mut self) {
        while let Some(cmd) = self.command_rx.recv().await {
            self.handle_command_full(cmd);
        }
    }

    // PERF: merge only call to_bytes once per BackboneDatagram
    // currently, it will be called once for every peer cascade
    // PERF: somewhat related, backbone.broadcast_datagram should ideally be batched
    fn handle_command_full(&mut self, command: CommandFull) {
        match command {
            CommandFull::Inner(inner) => self.handle_command(inner),
            CommandFull::MediaData(media) => {
                self.backbone
                    .broadcast_datagram(&[self.remote_sfu], BackboneDatagram::Media(media));
            }
            CommandFull::Speaking(speaking) => {
                self.backbone
                    .broadcast_datagram(&[self.remote_sfu], BackboneDatagram::Speaking(speaking));
            }
            CommandFull::NetworkPacket(_, _) => {
                warn!("cascade peers don't handle network packets directly")
            }
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::Signalling(_) => todo!("shouldn't be reachable?"),
            // Command::Signalling(cmd) => match cmd {
            //     SignallingCommand::Answer { .. } => todo!("handle sdp negotiation"),
            //     SignallingCommand::Offer { .. } => todo!("handle sdp negotiation"),
            //     SignallingCommand::Candidate { .. } => todo!("handle ice candidates"),
            //     _ => {}
            // },
            Command::GenerateKeyframe {
                mid,
                rid,
                kind,
                user_id,
            } => {
                let dispatch = BackboneDispatchEnvelope {
                    nonce: None,
                    dispatch: BackboneDispatch::Keyframe {
                        user_id,
                        mid,
                        rid,
                        kind,
                    },
                };

                if let Err(e) = self.backbone.send_dispatch(self.remote_sfu, dispatch) {
                    warn!("failed to queue keyframe dispatch to remote sfu: {:?}", e);
                }
            }
            Command::MediaAdded(_) => todo!("forward media addition to remote sfu"),
        }
    }
}

#[async_trait]
impl Peer for PeerCascading {
    fn id(&self) -> UserId {
        todo!("what would i return here?")
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
        self.event_rx.recv().await
    }
}
