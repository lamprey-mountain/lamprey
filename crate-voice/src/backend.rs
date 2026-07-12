use crate::prelude::*;

use common::v1::types::voice::messages::{SfuCommand, SfuEvent};
use futures_util::{SinkExt, StreamExt};
use lamprey_backend_core::config::Config;
use std::time::Duration;
use tokio::sync::{
    broadcast,
    mpsc::{self, UnboundedSender},
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};
use tracing::{debug, error, info, warn};

/// a connection to the api server
pub struct BackendConnection {
    config: Config,
    event_rx: mpsc::UnboundedReceiver<SfuEvent>,
    command_broadcast: broadcast::Sender<SfuCommand>,
}

/// a way to send events to the api server
#[derive(Clone)]
pub struct BackendHandle {
    event_tx: UnboundedSender<SfuEvent>,
    command_broadcast: broadcast::Sender<SfuCommand>,
}

impl BackendConnection {
    // PERF: take &Config, only clone whats needed, store necessary config in BackendConnection struct
    pub async fn connect(config: Config) -> Result<BackendHandle> {
        let (event_tx, event_rx) = mpsc::unbounded_channel::<SfuEvent>();
        let (command_broadcast, _) = broadcast::channel(100);

        let me = Self {
            config,
            event_rx,
            command_broadcast: command_broadcast.clone(),
        };

        let handle = BackendHandle {
            event_tx,
            command_broadcast,
        };

        tokio::spawn(me.run());

        Ok(handle)
    }

    async fn run(mut self) {
        let mut backoff = 1u64;
        loop {
            match self.reconnect().await {
                Err(e) => error!("Backend connection error: {}. Retrying with backoff.", e),
                Ok(_) => warn!("Disconnected from backend. Reconnecting with backoff."),
            }

            let jitter = rand::random::<u64>() % 3;
            tokio::time::sleep(Duration::from_secs(backoff + jitter)).await;
            backoff = std::cmp::min(backoff * 2, 30);
        }
    }

    async fn reconnect(&mut self) -> Result<()> {
        let url_str = format!("{}api/v1/internal/rpc", self.config.api_url)
            .replace("http", "ws")
            .replace("https", "wss");

        let voice_config = self.config.voice.clone().expect("voice config required");
        let token = voice_config.token.clone();

        let mut request = url_str
            .parse::<String>()
            .expect("infallible")
            .into_client_request()?;
        let auth_header = format!("Server {}", token)
            .try_into()
            .map_err(|e| Error::InvalidAuthToken(format!("{e}")))?;
        request.headers_mut().insert("Authorization", auth_header);

        info!("Connecting to backend websocket...");
        let (ws_stream, _) = connect_async(request).await?;
        info!("Connected to backend websocket");

        let (mut ws_tx, mut ws_rx) = ws_stream.split();

        loop {
            tokio::select! {
                Some(event) = self.event_rx.recv() => {
                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(e) => { error!("Failed to serialize event: {}", e); continue; }
                    };
                    if let Err(e) = ws_tx.send(Message::text(json)).await {
                        error!("Failed to send message to backend: {}", e);
                        return Err(Error::Channel(format!("Failed to send message to backend: {e}")));
                    }
                }
                Some(msg) = ws_rx.next() => {
                    match msg? {
                        Message::Text(t) => {
                            match serde_json::from_str::<SfuCommand>(&t) {
                                Ok(cmd) => {
                                    let _ = self.command_broadcast.send(cmd);
                                }
                                Err(e) => error!("Failed to deserialize command: {}", e),
                            }
                        }
                        Message::Close(_) => { info!("Backend websocket closed"); break; }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }
}

impl BackendHandle {
    pub fn send(&self, event: SfuEvent) -> Result<()> {
        debug!("send sfu event {event:?}");
        self.event_tx
            .send(event)
            .map_err(|e| Error::Channel(format!("Failed to queue event: {e}")))
    }

    pub fn subscribe(&self) -> impl Stream<Item = SfuCommand> {
        BroadcastStream::new(self.command_broadcast.subscribe())
            .filter_map(|r| async move { r.ok() })
    }
}
