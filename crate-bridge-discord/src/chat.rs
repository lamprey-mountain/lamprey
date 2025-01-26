use std::sync::Arc;

use anyhow::{Error, Result};
use tokio::sync::{mpsc, oneshot};
use tracing::info;
use types::{
    MediaCreated, MessageCreateRequest, MessageId, Session, Thread, ThreadId, User, UserId
};
use uuid::uuid;
use sdk::{Client, EventHandler, Http};

use crate::{
    common::{Globals, GlobalsTrait},
    portal::{Portal, PortalMessage},
};

pub struct Unnamed {
    globals: Arc<Globals>,
    recv: mpsc::Receiver<UnnamedMessage>,
    client: Client,
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

struct Handle {
    globals: Arc<Globals>,
}

impl EventHandler for Handle {
    type Error = Error;
    
    async fn ready(&mut self, _user: Option<User>, _session: Session) -> Result<()> {
        Ok(())
    }
    
    async fn upsert_thread(&mut self, _thread: Thread) -> Result<()> {
        info!("chat upsert thread");
        // TODO: what to do here?
        Ok(())
    }
    
    async fn upsert_message(&mut self, message: types::Message) -> Result<()> {
        info!("chat upsert message");
        if message.author.id == UserId(uuid!("01943cc1-62e0-7c0e-bb9b-a4ff42864d69")) {
            return Ok(());
    }
        self.globals.portal_send(
            message.thread_id,
            PortalMessage::UnnamedMessageUpsert { message },
        );
        Ok(())
    }
    
    async fn delete_message(&mut self, thread_id: ThreadId, message_id: MessageId) -> Result<()> {
        info!("chat delete message");
        self.globals.portal_send(
            thread_id,
            PortalMessage::UnnamedMessageDelete { message_id },
        );
        Ok(())
    }
}

impl Unnamed {
    pub fn new(globals: Arc<Globals>, recv: mpsc::Receiver<UnnamedMessage>) -> Self {
        let token = std::env::var("MY_TOKEN").expect("missing MY_TOKEN");
        let handle = Handle { globals: globals.clone() };
        let client = Client::new(token.clone().into()).with_handler(Box::new(handle));
        Self { globals, client, recv }
    }

    pub async fn connect(mut self) -> Result<()> {
        tokio::spawn(async move {
            while let Some(msg) = self.recv.recv().await {
                let _ = handle(msg, &self.client.http).await;
            }
        });
        
        let _ = self.client.syncer.connect().await;
        Ok(())
    }
}

async fn handle(msg: UnnamedMessage, http: &Http) -> Result<()> {
    let token = &http.token.to_string();
    match msg {
        UnnamedMessage::MediaUpload {
            filename,
            bytes,
            response,
        } => {
            // send_message
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
