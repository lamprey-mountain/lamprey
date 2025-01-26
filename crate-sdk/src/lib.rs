use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{error, warn};
use types::{
    MessageClient, MessageEnvelope, MessagePayload,
    MessageSync, SessionToken, SyncResume,
};

pub struct Client {
    handler: Box<dyn ErasedHandler>,
    token: SessionToken,
}

mod handler;

pub use handler::EventHandler;

struct EmptyHandler;

impl EventHandler for EmptyHandler {
    type Error = ();
}

#[async_trait]
pub trait ErasedHandler {
    async fn handle(&mut self, payload: MessagePayload);
}

#[async_trait]
impl<T, E> ErasedHandler for T
where
    T: EventHandler<Error = E>,
{
    async fn handle(&mut self, payload: MessagePayload) {
        let _ = match payload {
            MessagePayload::Sync { data, .. } => match data {
                MessageSync::UpsertRoom { room } => self.upsert_room(room).await,
                MessageSync::UpsertThread { thread } => self.upsert_thread(thread).await,
                MessageSync::UpsertMessage { message } => self.upsert_message(message).await,
                MessageSync::UpsertUser { user } => self.upsert_user(user).await,
                MessageSync::UpsertMember { member } => self.upsert_member(member).await,
                MessageSync::UpsertSession { session } => self.upsert_session(session).await,
                MessageSync::UpsertRole { role } => self.upsert_role(role).await,
                MessageSync::UpsertInvite { invite } => self.upsert_invite(invite).await,
                MessageSync::DeleteMessage {
                    thread_id,
                    message_id,
                } => self.delete_message(thread_id, message_id).await,
                MessageSync::DeleteMessageVersion {
                    thread_id,
                    message_id,
                    version_id,
                } => {
                    self.delete_message_version(thread_id, message_id, version_id)
                        .await
                }
                MessageSync::DeleteUser { id } => self.delete_user(id).await,
                MessageSync::DeleteSession { id } => self.delete_session(id).await,
                MessageSync::DeleteRole { room_id, role_id } => {
                    self.delete_role(room_id, role_id).await
                }
                MessageSync::DeleteMember { room_id, user_id } => {
                    self.delete_member(room_id, user_id).await
                }
                MessageSync::DeleteInvite { code } => self.delete_invite(code).await,
                MessageSync::Webhook { hook_id, data } => self.webhook(hook_id, data).await,
            },
            MessagePayload::Error { error } => self.error(error).await,
            MessagePayload::Ready { user, session, .. } => self.ready(user, session).await,
            _ => return,
        };
    }
}

impl Client {
    pub fn new(token: SessionToken) -> Self {
        Self { token, handler: Box::new(EmptyHandler) }
    }

    pub fn with_handler(self, handler: Box<dyn ErasedHandler>) -> Self {
        Self { handler, ..self }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let mut resume: Option<SyncResume> = None;
        loop {
            let Ok((mut client, _)) =
                tokio_tungstenite::connect_async("wss://chat.celery.eu.org/api/v1/sync").await
            else {
                warn!("websocket failed to connect, retrying in 1 second...");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            };
            let hello = MessageClient::Hello {
                token: self.token.clone(),
                resume: resume.clone(),
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
                    _ => {}
                }
                self.handler.handle(msg.payload).await;
            }
            warn!("websocket disconnected, reconnecting in 1 second...");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    // async fn send_message(&self) {
    //     // let c = reqwest::Client::new();
    //     // let res: types::MediaCreated = c
    //     //     .post("https://chat.celery.eu.org/api/v1/media")
    //     //     .bearer_auth(token)
    //     //     .header("content-type", "application/json")
    //     //     .json(&types::MediaCreate {
    //     //         filename,
    //     //         size: bytes.len() as u64,
    //     //         alt: None,
    //     //         url: None,
    //     //         source_url: None,
    //     //     })
    //     //     .send()
    //     //     .await?
    //     //     .error_for_status()?
    //     //     .json()
    //     //     .await?;
    //     // c.patch(res.upload_url.clone().unwrap())
    //     //     .bearer_auth(token)
    //     //     .header("upload-offset", "0")
    //     //     .body(bytes)
    //     //     .send()
    //     //     .await?
    //     //     .error_for_status()?;
    //     // let _ = response.send(res);
    //     // let mut h = HeaderMap::new();
    //     // h.insert("authorization", (&token).try_into().unwrap());
    //     // h.insert("content-type", "application/json".try_into().unwrap());
    //     // let c = reqwest::Client::builder().default_headers(h).build().unwrap();
    // }
}
