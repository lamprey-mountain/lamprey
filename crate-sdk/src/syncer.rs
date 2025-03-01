use std::time::Duration;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{error, warn};
use types::{MessageClient, MessageEnvelope, MessagePayload, SessionToken, SyncResume};

use crate::handler::{EmptyHandler, ErasedHandler};

pub struct Syncer {
    handler: Box<dyn ErasedHandler>,
    token: SessionToken,
    base_url: Url,
}

const DEFAULT_BASE: &str = "wss://chat.celery.eu.org/";

impl Syncer {
    pub fn new(token: SessionToken) -> Self {
        let base_url = Url::parse(DEFAULT_BASE).unwrap();
        Self {
            token,
            base_url,
            handler: Box::new(EmptyHandler),
        }
    }

    pub fn with_base_url(self, base_url: Url) -> Self {
        Self { base_url, ..self }
    }

    pub fn with_handler(self, handler: Box<dyn ErasedHandler>) -> Self {
        Self { handler, ..self }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let mut resume: Option<SyncResume> = None;
        loop {
            let url = self.base_url.join("/api/v1/sync")?;
            let Ok((mut client, _)) = tokio_tungstenite::connect_async(url.as_str()).await else {
                warn!("websocket failed to connect, retrying in 1 second...");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            };
            let hello = MessageClient::Hello {
                token: self.token.clone(),
                resume: resume.clone(),
                status: None,
            };
            client
                .send(WsMessage::text(serde_json::to_string(&hello)?))
                .await?;
            while let Some(Ok(msg)) = client.next().await {
                let WsMessage::Text(text) = msg else { continue };
                let msg: MessageEnvelope = serde_json::from_str(&text)?;
                match &msg.payload {
                    MessagePayload::Ping => {
                        client
                            .send(WsMessage::text(serde_json::to_string(
                                &MessageClient::Pong,
                            )?))
                            .await?;
                    }
                    MessagePayload::Error { error } => {
                        error!("{error}");
                    }
                    MessagePayload::Ready { conn, seq, .. } => {
                        resume = Some(SyncResume {
                            conn: conn.to_string(),
                            seq: *seq,
                        });
                    }
                    MessagePayload::Reconnect { can_resume } => {
                        if !can_resume {
                            resume = None;
                        }
                        client.close(None).await?;
                    }
                    MessagePayload::Sync { seq, .. } => {
                        if let Some(resume) = &mut resume {
                            resume.seq = *seq;
                        }
                    }
                    _ => {}
                }
                self.handler.handle(msg.payload).await;
            }
            warn!("websocket disconnected, reconnecting in 1 second...");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
