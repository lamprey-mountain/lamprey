use std::{
    sync::{atomic::AtomicU8, Arc},
    time::Duration,
};

use crate::prelude::*;
use common::v1::types::{
    presence::Presence, MessageClient, MessageEnvelope, MessagePayload, MessageSync, SessionToken,
    SyncResume,
};
use futures_util::{stream::BoxStream, SinkExt, StreamExt};
use reqwest::Url;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::tungstenite::{Error as WsError, Message as WsMessage};
use tracing::{debug, error, warn};

type WebSocketStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

pub struct Syncer {
    state: AtomicU8,
    client: Option<WebSocketStream>,
    resume: Option<SyncResume>,
    rx: mpsc::Receiver<SyncerCommand>,
    tx: broadcast::Sender<Arc<SyncerEvent>>,
}

/// a handle to a syncer
pub struct SyncerHandle {
    state: AtomicU8,
    tx: mpsc::Sender<SyncerCommand>,
    rx: broadcast::Receiver<Arc<SyncerEvent>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SyncerState {
    /// not connected; will make no attempt to
    Disconnected,

    /// delaying for a bit, will attemp to reconnect soon
    Waiting,

    /// waiting for websocket connection
    Connecting,

    /// sent hello, waiting for ready
    Authenticating,

    /// sent resume, waiting for resumed
    Resuming,

    /// connected and active
    Connected,
}

impl TryFrom<u8> for SyncerState {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Disconnected),
            1 => Ok(Self::Waiting),
            2 => Ok(Self::Connecting),
            3 => Ok(Self::Authenticating),
            4 => Ok(Self::Resuming),
            5 => Ok(Self::Connected),
            _ => Err(Error::Other("invalid syncer state".to_string())),
        }
    }
}

impl From<SyncerState> for u8 {
    fn from(value: SyncerState) -> Self {
        value as u8
    }
}

#[derive(Debug)]
pub enum SyncerCommand {
    /// send a sync message to the server
    Send(MessageClient),

    /// disconnect from the server
    Disconnect,

    /// reconnect to the server
    Connect,
}

#[derive(Debug, Clone)]
pub enum SyncerEvent {
    /// emitted whenever any message is received
    Message(Arc<MessageEnvelope>),

    /// emitted whenever message syncs are received
    Sync(Arc<MessageSync>),

    /// emitted when the syncer's connection state changes
    StateChanged,
}

#[derive(Default)]
pub struct SyncerBuilder {
    sync_url: Option<Url>,
    token: Option<SessionToken>,
    presence: Option<Presence>,
}

impl SyncerBuilder {
    pub fn sync_url(mut self, url: Url) -> Self {
        self.sync_url = Some(url);
        self
    }

    pub fn token(mut self, token: SessionToken) -> Self {
        self.token = Some(token);
        self
    }

    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = Some(presence);
        self
    }

    pub async fn connect(self) -> Result<SyncerHandle> {
        let (cmd_tx, cmd_rx) = mpsc::channel(100);
        let (evt_tx, _) = broadcast::channel(100);

        let syncer = Syncer {
            state: AtomicU8::new(SyncerState::Connecting.into()),
            client: None,
            resume: None,
            rx: cmd_rx,
            tx: evt_tx.clone(),
        };

        let token = self
            .token
            .ok_or_else(|| Error::MissingBuilderField("token".to_string()))?;
        let base_url = self
            .sync_url
            .ok_or_else(|| Error::MissingBuilderField("sync_url".to_string()))?;

        tokio::spawn(syncer.run(token, base_url));

        Ok(SyncerHandle {
            state: AtomicU8::new(SyncerState::Connecting.into()),
            tx: cmd_tx,
            rx: evt_tx.subscribe(),
        })
    }
}

enum Propagated {
    Noop,
    Disconnected,
}

impl Syncer {
    pub fn builder() -> SyncerBuilder {
        todo!()
    }

    fn state(&self) -> SyncerState {
        self.state
            .load(std::sync::atomic::Ordering::SeqCst)
            .try_into()
            .unwrap_or(SyncerState::Disconnected)
    }

    async fn run(mut self, token: SessionToken, base_url: Url) {
        loop {
            if self.state() == SyncerState::Disconnected {
                match self.rx.recv().await {
                    Some(SyncerCommand::Connect) => {
                        self.set_state(SyncerState::Connecting).ok();
                    }
                    Some(_) => continue,
                    // all handles dropped
                    None => return,
                }
            }

            let url = base_url
                .join("/api/v1/sync?version=1")
                .expect("invalid url");

            let (client, _) = match tokio_tungstenite::connect_async(url.as_str()).await {
                Ok(res) => res,
                Err(_) => {
                    warn!("websocket failed to connect, retrying in 1 second...");
                    self.set_state(SyncerState::Waiting).ok();
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                        cmd = self.rx.recv() => {
                            match cmd {
                                Some(SyncerCommand::Disconnect) => {
                                    self.set_state(SyncerState::Disconnected).ok();
                                }
                                Some(SyncerCommand::Connect) => {}
                                Some(SyncerCommand::Send(_)) => {
                                    // TODO: queue Send commands
                                    warn!("ignoring send while waiting");
                                }
                                None => return,
                            }
                        }
                    }
                    continue;
                }
            };

            self.client = Some(client);
            self.set_state(SyncerState::Authenticating).ok();

            if let Err(e) = self
                .send(MessageClient::Hello {
                    token: token.clone(),
                    resume: self.resume.clone(),
                    presence: None,
                })
                .await
            {
                error!("failed to send hello: {e}");
                self.client = None;
                continue;
            }

            loop {
                match self.poll().await {
                    Propagated::Noop => {}
                    Propagated::Disconnected => {
                        break;
                    }
                }
            }

            self.client = None;
            if self.state() == SyncerState::Disconnected {
                continue;
            }

            warn!("websocket disconnected, retrying in 1 second...");
            self.set_state(SyncerState::Waiting).ok();
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                cmd = self.rx.recv() => {
                    match cmd {
                        Some(SyncerCommand::Disconnect) => {
                            // TODO: queue Send commands
                            self.set_state(SyncerState::Disconnected).ok();
                        }
                        Some(SyncerCommand::Connect) => {}
                        Some(SyncerCommand::Send(_)) => {
                            warn!("ignoring send while waiting");
                        }
                        None => return,
                    }
                }
            }
        }
    }

    fn client(&mut self) -> &mut WebSocketStream {
        self.client.as_mut().unwrap()
    }

    async fn poll(&mut self) -> Propagated {
        tokio::select! {
            msg = self.client.as_mut().expect("no client").next() => self.handle_websocket_event(msg).await,
            Some(cmd) = self.rx.recv() => self.handle_command(cmd).await,
        }
    }

    async fn handle_websocket_event(
        &mut self,
        event: Option<core::result::Result<WsMessage, WsError>>,
    ) -> Propagated {
        match event {
            Some(Ok(msg)) => {
                if let Err(err) = self.handle_message(msg).await {
                    error!("handle_message error: {err}");
                    Propagated::Noop
                } else {
                    Propagated::Noop
                }
            }
            Some(Err(err)) => match err {
                // TODO: better error handling
                // WsError::ConnectionClosed => todo!(),
                // WsError::AlreadyClosed => todo!(),
                // WsError::Io(error) => todo!(),
                // WsError::Tls(tls_error) => todo!(),
                // WsError::Capacity(capacity_error) => todo!(),
                // WsError::Protocol(protocol_error) => todo!(),
                // WsError::WriteBufferFull(message) => todo!(),
                // WsError::Utf8(_) => todo!(),
                // WsError::AttackAttempt => todo!(),
                // WsError::Url(url_error) => todo!(),
                // WsError::Http(response) => todo!(),
                // WsError::HttpFormat(error) => todo!(),
                err => {
                    error!("websocket error: {err}");
                    Propagated::Disconnected
                }
            },
            None => Propagated::Disconnected,
        }
    }

    async fn handle_message(&mut self, msg: WsMessage) -> Result<()> {
        let WsMessage::Text(text) = msg else {
            return Ok(());
        };

        let msg: MessageEnvelope = serde_json::from_str(&text)?;
        debug!("got lamprey message {msg:?}");
        match &msg.payload {
            MessagePayload::Ping => {
                self.send(MessageClient::Pong).await?;
            }
            MessagePayload::Error { error, code } => {
                if let Some(code) = code {
                    error!("sync error [{code:?}]: {error}");
                } else {
                    error!("sync error: {error}");
                }
            }
            MessagePayload::Ready { conn, seq, .. } => {
                self.set_state(SyncerState::Connected)?;
                self.resume = Some(SyncResume {
                    conn: *conn,
                    seq: *seq,
                });
            }
            MessagePayload::Reconnect { can_resume } => {
                if !can_resume {
                    self.resume = None;
                }

                // NOTE: should i specify a close frame?
                self.client().close(None).await?;
            }
            MessagePayload::Sync { seq, .. } => {
                if let Some(resume) = &mut self.resume {
                    resume.seq = *seq;
                }
            }
            MessagePayload::Resumed => {
                self.set_state(SyncerState::Connected)?;
            }
        }

        self.emit(SyncerEvent::Message(Arc::new(msg)))?;

        Ok(())
    }

    async fn handle_command(&mut self, cmd: SyncerCommand) -> Propagated {
        match cmd {
            SyncerCommand::Send(m) => {
                if let Err(e) = self.send(m).await {
                    error!("failed to send message: {e}");
                }
            }
            SyncerCommand::Disconnect => {
                self.set_state(SyncerState::Disconnected).ok();
                if let Some(mut client) = self.client.take() {
                    let _ = client.close(None).await;
                }
                return Propagated::Disconnected;
            }
            SyncerCommand::Connect => {
                debug!("reconnect command received");
                self.set_state(SyncerState::Connecting).ok();
                if let Some(mut client) = self.client.take() {
                    let _ = client.close(None).await;
                }
                return Propagated::Disconnected;
            }
        }

        Propagated::Noop
    }

    fn emit(&self, msg: SyncerEvent) -> Result<()> {
        // TODO: error handling
        let _ = self.tx.send(Arc::new(msg));
        Ok(())
    }

    async fn send(&mut self, msg: MessageClient) -> Result<()> {
        let text = serde_json::to_string(&msg)?;
        // TODO: proper error variant
        self.client()
            .send(WsMessage::text(text))
            .await
            .map_err(|e| Error::Other(e.to_string()))
    }

    fn set_state(&self, state: SyncerState) -> Result<()> {
        self.state
            .store(state.into(), std::sync::atomic::Ordering::SeqCst);
        self.emit(SyncerEvent::StateChanged)?;
        Ok(())
    }
}

impl SyncerHandle {
    /// get the syncer's current state
    pub fn state(&self) -> SyncerState {
        self.state
            .load(std::sync::atomic::Ordering::SeqCst)
            .try_into()
            .unwrap_or(SyncerState::Disconnected)
    }

    /// get a stream of events
    pub fn subscribe(&self) -> BoxStream<'static, Arc<SyncerEvent>> {
        BroadcastStream::new(self.rx.resubscribe())
            .filter_map(|e| async { e.ok() })
            .boxed()
    }

    /// get a stream of message syncs
    pub fn sync(&self) -> BoxStream<'static, Arc<MessageSync>> {
        self.subscribe()
            .filter_map(|e| async move {
                match &*e {
                    SyncerEvent::Sync(m) => Some(Arc::clone(m)),
                    _ => None,
                }
            })
            .boxed()
    }

    /// attempt to disconnect the syncer
    pub fn disconnect(&self) {
        let _ = self.tx.try_send(SyncerCommand::Disconnect);
    }

    /// attempt to reconnect the syncer
    pub fn connect(&self) {
        let _ = self.tx.try_send(SyncerCommand::Connect);
    }
}
