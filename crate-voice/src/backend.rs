//! connection to the backend/master

use crate::sfu::State;
use anyhow::Result;
use common::v1::types::voice::messages::{SfuCommand, SfuEvent};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};
use tracing::{error, info, warn};

pub struct BackendConnection {
    event_tx: UnboundedSender<SfuEvent>,
    command_rx: UnboundedReceiver<SfuCommand>,
}

impl BackendConnection {
    /// connect to the server and start receiving events
    pub async fn connect(state: State) -> Result<Self> {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<SfuEvent>();
        let (command_tx, command_rx) = mpsc::unbounded_channel::<SfuCommand>();

        let url_str = format!("{}/api/v1/internal/rpc", state.config.api_url)
            .replace("http", "ws")
            .replace("https", "wss");

        let token = state.voice_config.token.clone();

        tokio::spawn(async move {
            let mut backoff = 1u64;
            loop {
                let result: Result<()> = async {
                    let mut request = url_str.parse::<String>()?.into_client_request()?;
                    let auth_header = format!("Server {}", token)
                        .try_into()
                        .map_err(|e| anyhow::anyhow!("Invalid auth token: {e}"))?;
                    request.headers_mut().insert("Authorization", auth_header);

                    info!("Connecting to backend websocket...");
                    let (ws_stream, _) = connect_async(request).await?;
                    info!("Connected to backend websocket");
                    backoff = 1;

                    let (mut ws_tx, mut ws_rx) = ws_stream.split();

                    loop {
                        tokio::select! {
                            Some(event) = event_rx.recv() => {
                                let json = match serde_json::to_string(&event) {
                                    Ok(j) => j,
                                    Err(e) => { error!("Failed to serialize event: {}", e); continue; }
                                };
                                if let Err(e) = ws_tx.send(Message::text(json)).await {
                                    error!("Failed to send message to backend: {}", e);
                                    break;
                                }
                            }
                            Some(msg) = ws_rx.next() => {
                                match msg? {
                                    Message::Text(t) => {
                                        match serde_json::from_str::<SfuCommand>(&t) {
                                            Ok(cmd) => { let _ = command_tx.send(cmd); }
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
                }.await;

                match result {
                    Err(e) => error!("Backend connection error: {}. Retrying with backoff.", e),
                    Ok(_) => warn!("Disconnected from backend. Reconnecting with backoff."),
                }

                let jitter = rand::random::<u64>() % 3;
                tokio::time::sleep(Duration::from_secs(backoff + jitter)).await;
                backoff = std::cmp::min(backoff * 2, 30);
            }
        });

        Ok(Self {
            event_tx,
            command_rx,
        })
    }

    pub async fn poll(&mut self) -> Result<SfuCommand> {
        self.command_rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Backend command channel closed"))
    }

    pub async fn send(&self, event: SfuEvent) -> Result<()> {
        self.event_tx
            .send(event)
            .map_err(|e| anyhow::anyhow!("Failed to queue event: {e}"))
    }
}
