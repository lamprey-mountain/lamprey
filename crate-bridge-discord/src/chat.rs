use std::{sync::Arc, time::Duration};

use anyhow::Result;
use serenity::futures::{SinkExt as _, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};
use types::{
    MediaCreated, MessageClient, MessageCreateRequest, MessageId, MessageServer, ThreadId, UserId,
};
use uuid::uuid;

use crate::common::{Globals, GlobalsTrait, PortalMessage};

pub struct Unnamed {
    globals: Arc<Globals>,
    recv: mpsc::Receiver<UnnamedMessage>,
}

pub enum UnnamedMessage {
    MediaUpload {
        filename: String,
        bytes: Vec<u8>,
        response: oneshot::Sender<MediaCreated>,
    },
    MessageGet {
        thread_id: ThreadId,
        message_id: MessageId,
        response: oneshot::Sender<types::Message>,
    },
    MessageCreate {
        thread_id: ThreadId,
        req: MessageCreateRequest,
        response: oneshot::Sender<types::Message>,
    },
    MessageUpdate {
        thread_id: ThreadId,
        message_id: MessageId,
        req: types::MessagePatch,
        response: oneshot::Sender<types::Message>,
    },
    MessageDelete {
        thread_id: ThreadId,
        message_id: MessageId,
        response: oneshot::Sender<()>,
    },
}

impl Unnamed {
    pub fn new(globals: Arc<Globals>, recv: mpsc::Receiver<UnnamedMessage>) -> Self {
        Self { globals, recv }
    }

    pub async fn connect(mut self) -> Result<()> {
        let token = std::env::var("MY_TOKEN").expect("missing MY_TOKEN");
        // let mut h = HeaderMap::new();
        // h.insert("authorization", (&token).try_into().unwrap());
        // h.insert("content-type", "application/json".try_into().unwrap());
        // let c = reqwest::Client::builder().default_headers(h).build().unwrap();
        let t2 = token.clone();
        tokio::spawn(async move {
            while let Some(msg) = self.recv.recv().await {
                let _ = handle(msg, &t2).await;
            }
        });
        loop {
            let Ok((mut client, _)) =
                tokio_tungstenite::connect_async("wss://chat.celery.eu.org/api/v1/sync").await
            else {
                warn!("websocket failed to connect, retrying in 1 second...");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            };
            let hello = types::MessageClient::Hello {
                token: token.clone(),
                last_id: None,
            };
            client
                .send(Message::text(serde_json::to_string(&hello)?))
                .await?;
            while let Some(Ok(msg)) = client.next().await {
                let Message::Text(text) = msg else { continue };
                let msg: MessageServer = serde_json::from_str(&text)?;
                match msg {
                    MessageServer::Ping {} => {
                        client
                            .send(Message::text(serde_json::to_string(&MessageClient::Pong)?))
                            .await?;
                    }
                    MessageServer::Ready { user } => {
                        info!("chat ready {}", user.name);
                    }
                    MessageServer::UpsertThread { thread: _ } => {
                        info!("chat upsert thread");
                        // TODO: what to do here?
                    }
                    MessageServer::UpsertMessage { message } => {
                        info!("chat upsert message");
                        if message.author.id
                            == UserId(uuid!("01943cc1-62e0-7c0e-bb9b-a4ff42864d69"))
                        {
                            continue;
                        }
                        self.globals.portal_send(
                            message.thread_id,
                            PortalMessage::UnnamedMessageUpsert { message },
                        );
                    }
                    MessageServer::DeleteMessage {
                        thread_id,
                        message_id,
                    } => {
                        info!("chat delete message");
                        self.globals.portal_send(
                            thread_id,
                            PortalMessage::UnnamedMessageDelete { message_id },
                        );
                    }
                    _ => {}
                }
            }
            warn!("websocket disconnected, reconnecting in 1 second...");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

async fn handle(msg: UnnamedMessage, token: &str) -> Result<()> {
    match msg {
        UnnamedMessage::MediaUpload {
            filename,
            bytes,
            response,
        } => {
            let c = reqwest::Client::new();
            let res: types::MediaCreated = c
                .post("https://chat.celery.eu.org/api/v1/media")
                .header("authorization", token)
                .header("content-type", "application/json")
                .json(&types::MediaCreate {
                    filename,
                    size: bytes.len() as u64,
                    alt: None,
                    url: None,
                    source_url: None,
                })
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            c.patch(res.upload_url.clone().unwrap())
                .header("authorization", token)
                .header("upload-offset", "0")
                .body(bytes)
                .send()
                .await?
                .error_for_status()?;
            let _ = response.send(res);
        }
        UnnamedMessage::MessageCreate {
            thread_id,
            req,
            response,
        } => {
            let c = reqwest::Client::new();
            let url = format!("https://chat.celery.eu.org/api/v1/thread/{thread_id}/message");
            let res: types::Message = c
                .post(url)
                .header("authorization", token)
                .header("content-type", "application/json")
                .json(&req)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let _ = response.send(res);
        }
        UnnamedMessage::MessageUpdate {
            thread_id,
            message_id,
            req,
            response,
        } => {
            let c = reqwest::Client::new();
            let url = format!(
                "https://chat.celery.eu.org/api/v1/thread/{thread_id}/message/{message_id}"
            );
            let res: types::Message = c
                .patch(url)
                .header("authorization", token)
                .header("content-type", "application/json")
                .json(&req)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let _ = response.send(res);
        }
        UnnamedMessage::MessageDelete {
            thread_id,
            message_id,
            response,
        } => {
            let c = reqwest::Client::new();
            let url = format!(
                "https://chat.celery.eu.org/api/v1/thread/{thread_id}/message/{message_id}"
            );
            c.delete(url)
                .header("authorization", token)
                .send()
                .await?
                .error_for_status()?;
            let _ = response.send(());
        }
        UnnamedMessage::MessageGet {
            thread_id,
            message_id,
            response,
        } => {
            let url = format!(
                "https://chat.celery.eu.org/api/v1/thread/{}/message/{}",
                thread_id, message_id
            );
            let message: types::Message = reqwest::Client::new()
                .get(url)
                .header("authorization", token)
                .header("content-type", "application/json")
                .send()
                .await?
                .json()
                .await?;
            let _ = response.send(message);
        }
    }
    Ok(())
}
