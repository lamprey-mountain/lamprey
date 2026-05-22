//! connection to the backend/master

use anyhow::Result;
use common::v1::types::voice::messages::{SfuCommand, SfuEvent};
use futures_util::{SinkExt, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};
use tracing::{error, info, warn};

use crate::sfu::State;

pub struct BackendConnection {
    state: State,
}

impl BackendConnection {
    /// connect to the server and start receiving events
    pub async fn connect(state: State) -> Result<Self> {
        todo!()
    }

    pub async fn poll(&self) -> Result<SfuCommand> {
        todo!()
    }

    pub async fn send(&self, command: SfuEvent) -> Result<()> {
        todo!()
    }
}

// OLD CODE BELOW

// pub struct BackendConnection {
//     config: Arc<Config>,
//     event_rx: UnboundedReceiver<SfuEvent>,
//     command_tx: UnboundedSender<SfuCommand>,
// }

// impl BackendConnection {
//     pub fn new(
//         config: Arc<Config>,
//         event_rx: UnboundedReceiver<SfuEvent>,
//         command_tx: UnboundedSender<SfuCommand>,
//     ) -> Self {
//         Self {
//             config,
//             event_rx,
//             command_tx,
//         }
//     }

//     pub async fn spawn(mut self) {
//         let mut backoff = 1;
//         loop {
//             if let Err(e) = self.connect_and_run().await {
//                 error!("Backend connection error: {}. Retrying with backoff.", e);
//             } else {
//                 warn!("Disconnected from backend. Reconnecting with backoff.");
//             }

//             // Exponential backoff with a cap of 30 seconds
//             let jitter = rand::random::<u64>() % 3;
//             tokio::time::sleep(Duration::from_secs(backoff + jitter)).await;
//             backoff = std::cmp::min(backoff * 2, 30);
//         }
//     }

//     async fn connect_and_run(&mut self) -> Result<()> {
//         let url_str = format!("{}/api/v1/internal/rpc", self.config.api_url)
//             .replace("http", "ws")
//             .replace("https", "wss");

//         let mut request = url_str.into_client_request()?;
//         let token = &self
//             .config
//             .voice
//             .as_ref()
//             .expect("can't use voice server with no token")
//             .token;
//         let auth_header = format!("Server {}", token)
//             .try_into()
//             .map_err(|e| anyhow::anyhow!("Invalid auth token: {e}"))?;

//         request.headers_mut().insert("Authorization", auth_header);

//         info!("Connecting to backend websocket...");
//         let (ws_stream, _) = connect_async(request).await?;
//         info!("Connected to backend websocket");

//         let (mut ws_tx, mut ws_rx) = ws_stream.split();

//         loop {
//             tokio::select! {
//                 Some(event) = self.event_rx.recv() => {
//                     let json = match serde_json::to_string(&event) {
//                         Ok(j) => j,
//                         Err(e) => {
//                             error!("Failed to serialize event: {}", e);
//                             continue;
//                         }
//                     };
//                     if let Err(e) = ws_tx.send(Message::text(json)).await {
//                         error!("Failed to send message to backend: {}", e);
//                         break;
//                     }
//                 }
//                 Some(msg) = ws_rx.next() => {
//                     match msg? {
//                         Message::Text(t) => {
//                             match serde_json::from_str::<SfuCommand>(&t) {
//                                 Ok(command) => {
//                                     if let Err(e) = self.command_tx.send(command) {
//                                         error!("Failed to send command to SFU: {}", e);
//                                     }
//                                 }
//                                 Err(e) => {
//                                     error!("Failed to deserialize command: {}", e);
//                                 }
//                             }
//                         }
//                         Message::Close(_) => {
//                             info!("Backend websocket closed");
//                             break;
//                         }
//                         _ => {}
//                     }
//                 }
//             }
//         }

//         Ok(())
//     }
// }
