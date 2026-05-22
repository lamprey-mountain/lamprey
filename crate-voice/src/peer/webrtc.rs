use std::{sync::Arc, time::Instant};

use async_trait::async_trait;
use common::v1::types::{
    voice::{
        internal::MediaData,
        messages::{PeerEvent, SignallingCommand},
        SpeakingWithPeerId, VoiceState,
    },
    PeerId,
};
use str0m::{Event, Input, Output, Rtc};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep_until;
use tracing::debug;

use crate::peer::{Command, CommandFull, Peer};

/// a handle to a webrtc peer connection
#[derive(Debug)]
pub struct PeerWebrtc {
    id: PeerId,
    command_tx: mpsc::UnboundedSender<CommandFull>,
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<PeerEvent>>>,
}

/// the actor responsible for the webrtc connection lifecycle
pub struct PeerWebrtcInner {
    id: PeerId,

    // NOTE: should i Box these?
    /// rtc instance for incoming media
    rtc_incoming: Rtc,

    /// rtc instance for outgoing media
    rtc_outgoing: Rtc,

    voice_state: VoiceState,
    command_rx: mpsc::UnboundedReceiver<CommandFull>,
    event_tx: mpsc::UnboundedSender<PeerEvent>,
}

impl PeerWebrtc {
    pub fn spawn(id: PeerId, voice_state: VoiceState) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let inner = PeerWebrtcInner {
            id,
            rtc_incoming: Rtc::new(),
            rtc_outgoing: Rtc::new(),
            voice_state,
            command_rx,
            event_tx,
        };

        tokio::spawn(async move {
            inner.run().await;
        });

        Self {
            id,
            command_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
        }
    }
}

impl PeerWebrtcInner {
    async fn run(mut self) {
        loop {
            // Poll both RTCs
            let timeout_incoming = match self.rtc_incoming.poll_output().unwrap() {
                Output::Timeout(t) => t,
                Output::Transmit(_) => Instant::now(),
                Output::Event(e) => {
                    self.handle_rtc_event(e, true);
                    Instant::now()
                }
            };
            let timeout_outgoing = match self.rtc_outgoing.poll_output().unwrap() {
                Output::Timeout(t) => t,
                Output::Transmit(_) => Instant::now(),
                Output::Event(e) => {
                    self.handle_rtc_event(e, false);
                    Instant::now()
                }
            };

            let timeout = std::cmp::min(timeout_incoming, timeout_outgoing);

            tokio::select! {
                Some(cmd) = self.command_rx.recv() => {
                    self.handle_command_full(cmd);
                }
                _ = sleep_until(timeout.into()) => {
                    self.rtc_incoming.handle_input(Input::Timeout(Instant::now())).unwrap();
                    self.rtc_outgoing.handle_input(Input::Timeout(Instant::now())).unwrap();
                }
            }
        }
    }

    fn handle_rtc_event(&mut self, event: Event, incoming: bool) {
        // TODO: handle `incoming` correctly
        match event {
            Event::Connected => debug!("connected!"),

            Event::MediaAdded(_) => todo!("register media"),
            Event::MediaChanged(_) => todo!("register media"),
            Event::MediaData(_) => todo!("forward media to other peers"),
            Event::KeyframeRequest(_) => todo!("send keyframe request to backend"),

            // handle channel data (speaking)
            Event::ChannelOpen(_, _) => todo!("handle channel open"),
            Event::ChannelData(_) => todo!("handle channel data"),
            Event::ChannelClose(_) => todo!("handle channel close"),

            _ => {}
        }
    }

    fn handle_command_full(&mut self, command: CommandFull) {
        match command {
            CommandFull::Inner(command) => self.handle_command(command),
            CommandFull::MediaData(m) => todo!("forward to connected peer/client"),
            CommandFull::Speaking(s) => todo!("forward to connected peer/client"),
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::Signalling(cmd) => match cmd {
                SignallingCommand::Answer { .. } => todo!("handle sdp negotiation"),
                SignallingCommand::Offer { .. } => todo!("handle sdp negotiation"),
                SignallingCommand::Candidate { candidate } => {
                    if let Ok(c) = str0m::Candidate::from_sdp_string(&candidate) {
                        self.rtc_outgoing.add_remote_candidate(c);
                    }
                }
                SignallingCommand::Disconnect => todo!("disconnect"),
                SignallingCommand::VoiceState { state } => todo!("update voice state"),
                SignallingCommand::Want { subscriptions } => todo!("updase subscriptions"),
                SignallingCommand::Keyframe { mid, rid, kind } => {
                    todo!("send keyframe request to source peer")
                }
            },
            Command::GenerateKeyframe { mid, rid, kind } => {
                if let Some(mut w) = self.rtc_outgoing.writer(mid.into()) {
                    _ = w.request_keyframe(rid.map(|r| r.into()), kind.into());
                }
            }
            Command::MediaAdded(_) => {
                todo!("tell local peer about media, update mapping if needed")
            }
        }
    }

    // pub fn deliver_packet(source, destination, data);
}

#[async_trait]
impl Peer for PeerWebrtc {
    fn id(&self) -> PeerId {
        self.id
    }

    fn handle_command(&self, cmd: Command) {
        _ = self.command_tx.send(CommandFull::Inner(cmd));
    }

    fn handle_media_data(&self, media: MediaData) {
        _ = self.command_tx.send(CommandFull::MediaData(media));
    }

    fn handle_speaking(&self, speaking: SpeakingWithPeerId) {
        _ = self.command_tx.send(CommandFull::Speaking(speaking));
    }

    async fn poll(&mut self) -> Option<PeerEvent> {
        self.event_rx.lock().await.recv().await
    }
}
