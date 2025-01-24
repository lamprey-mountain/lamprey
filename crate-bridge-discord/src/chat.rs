use std::{sync::Arc, time::Duration};

use anyhow::Result;
use serenity::futures::{SinkExt as _, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};
use types::{
    MediaCreated, MessageClient, MessageCreateRequest, MessageEnvelope, MessageId, MessagePayload, MessageSync, PaginationResponse, SessionToken, SyncResume, ThreadId, UserId
};
use uuid::uuid;

use crate::{
    common::{Globals, GlobalsTrait},
    portal::{Portal, PortalMessage},
};

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

        let mut resume: Option<SyncResume> = None;
        loop {
            let Ok((mut client, _)) =
                tokio_tungstenite::connect_async("wss://chat.celery.eu.org/api/v1/sync").await
            else {
                warn!("websocket failed to connect, retrying in 1 second...");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            };
            let hello = types::MessageClient::Hello {
                token: SessionToken(token.clone()),
                resume: resume.clone(),
            };
            client
                .send(Message::text(serde_json::to_string(&hello)?))
                .await?;
            while let Some(Ok(msg)) = client.next().await {
                let Message::Text(text) = msg else { continue };
                let msg: MessageEnvelope = serde_json::from_str(&text)?;
                match msg.payload {
                    MessagePayload::Ping => {
                        client
                            .send(Message::text(serde_json::to_string(&MessageClient::Pong)?))
                            .await?;
                    }
                    MessagePayload::Sync { data, seq } => {
                        handle_sync(self.globals.clone(), data).await?;
                        if let Some(r) = &mut resume {
                            r.seq = seq;
                        }
                    }
                    MessagePayload::Error { error } => {
                        error!("{error}");
                    }
                    MessagePayload::Ready { user, conn, seq } => {
                        info!("chat ready {}", user.expect("tried to use unauthenticated sesion token!").name);

                        let http = reqwest::Client::new();
                        for config in &self.globals.config.portal {
                            let portal = self
                                .globals
                                .portals
                                .entry(config.my_thread_id)
                                .or_insert_with(|| {
                                    Portal::summon(self.globals.clone(), config.to_owned())
                                });
                            let last_id = self
                                .globals
                                .last_ids
                                .get(&config.my_thread_id)
                                .map(|m| m.chat_id);
                            let Some(mut last_id) = last_id else {
                                continue;
                            };
                            loop {
                                let url = format!("https://chat.celery.eu.org/api/v1/thread/{}/message?from={}&dir=f&limit=100", config.my_thread_id, last_id);
                                let batch: PaginationResponse<types::Message> = http
                                    .get(url)
                                    .bearer_auth(token.clone())
                                    .send()
                                    .await?
                                    .error_for_status()?
                                    .json()
                                    .await?;
                                info!("chat backfill {} messages", batch.items.len());
                                let new_last_id = batch.items.last().map(|m| m.id);
                                for message in batch.items.into_iter() {
                                    let _ = portal
                                        .send(PortalMessage::UnnamedMessageUpsert { message });
                                }
                                if !batch.has_more {
                                    break;
                                }
                                last_id = new_last_id.unwrap();
                            }
                        }

                        resume = Some(SyncResume { conn, seq });
                    }
                    MessagePayload::Resumed => {}
                    MessagePayload::Reconnect { can_resume } => {
                        if !can_resume {
                            resume = None;
                        }
                        client.close(None).await?;
                    }
                }
            }
            warn!("websocket disconnected, reconnecting in 1 second...");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

async fn handle_sync(mut globals: Arc<Globals>, msg: MessageSync) -> Result<()> {
    match msg {
        MessageSync::UpsertThread { thread: _ } => {
            info!("chat upsert thread");
            // TODO: what to do here?
        }
        MessageSync::UpsertMessage { message } => {
            info!("chat upsert message");
            if message.author.id == UserId(uuid!("01943cc1-62e0-7c0e-bb9b-a4ff42864d69")) {
                return Ok(());
            }
            globals.portal_send(
                message.thread_id,
                PortalMessage::UnnamedMessageUpsert { message },
            );
        }
        MessageSync::DeleteMessage {
            thread_id,
            message_id,
        } => {
            info!("chat delete message");
            globals.portal_send(
                thread_id,
                PortalMessage::UnnamedMessageDelete { message_id },
            );
        }
        _ => {}
    }
    Ok(())
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
                .bearer_auth(token)
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
                .bearer_auth(token)
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
                .bearer_auth(token)
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
                .bearer_auth(token)
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
                .bearer_auth(token)
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
                .bearer_auth(token)
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
