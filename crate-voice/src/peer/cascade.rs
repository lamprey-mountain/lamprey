use std::sync::Arc;

use async_trait::async_trait;
use common::v1::types::{
    voice::{
        internal::MediaData,
        messages::{PeerEvent, SignallingCommand},
        SpeakingWithUserId,
    },
    SfuId, UserId,
};
use tokio::sync::mpsc;
use tracing::debug;

use crate::{
    backbone::BackboneComms,
    peer::{Command, Peer},
};

/// a handle to a cascaded peer connection
#[derive(Debug)]
pub struct PeerCascading {
    id: UserId,
    command_tx: mpsc::UnboundedSender<Command>,
    event_rx: mpsc::UnboundedReceiver<PeerEvent>,
}

/// the actor responsible for the cascade lifecycle
pub struct PeerCascadingInner {
    id: UserId,
    remote_sfu: SfuId,
    backbone: Arc<BackboneComms>,
    command_rx: mpsc::UnboundedReceiver<Command>,
    event_tx: mpsc::UnboundedSender<PeerEvent>,
}

impl PeerCascading {
    pub fn spawn(id: UserId, remote_sfu: SfuId, backbone: Arc<BackboneComms>) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let inner = PeerCascadingInner {
            id,
            remote_sfu,
            backbone,
            command_rx,
            event_tx,
        };

        tokio::spawn(async move {
            inner.run().await;
        });

        Self {
            id,
            command_tx,
            event_rx,
        }
    }
}

impl PeerCascadingInner {
    async fn run(mut self) {
        while let Some(cmd) = self.command_rx.recv().await {
            self.handle_command(cmd);
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            // Command::Signalling(cmd) => todo!("shouldn't be reachable?"),
            Command::Signalling(cmd) => match cmd {
                SignallingCommand::Answer { .. } => todo!("handle sdp negotiation"),
                SignallingCommand::Offer { .. } => todo!("handle sdp negotiation"),
                SignallingCommand::Candidate { .. } => todo!("handle ice candidates"),
                _ => {}
            },
            Command::GenerateKeyframe { .. } => todo!("forward keyframe request"),
            Command::MediaAdded(_) => todo!("forward media addition"),
        }
    }
}

#[async_trait]
impl Peer for PeerCascading {
    fn id(&self) -> UserId {
        self.id
    }

    fn handle_command(&self, cmd: Command) {
        _ = self.command_tx.send(cmd);
    }

    fn handle_media_data(&self, _media: MediaData) {
        // TODO: Backbone datagram transmission
    }

    fn handle_speaking(&self, _speaking: SpeakingWithUserId) {
        // TODO: Backbone datagram transmission
    }

    fn handle_network_packet(&self, _source: std::net::SocketAddr, _data: bytes::Bytes) {
        // Cascaded peers don't handle raw network packets directly
    }

    async fn poll(&mut self) -> Option<PeerEvent> {
        self.event_rx.recv().await
    }
}
