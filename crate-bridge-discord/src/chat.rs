use std::sync::Arc;

use anyhow::{Error, Result};
use sdk::{Client, EventHandler, Http};
use tokio::sync::{mpsc, oneshot};
use tracing::info;
use types::{
    Media, MediaCreate, MessageCreateRequest, MessageId, Session, Thread, ThreadId, User, UserId,
};
use uuid::uuid;

use crate::{
    common::{Globals, GlobalsTrait},
    portal::PortalMessage,
};

pub struct Unnamed {
    recv: mpsc::Receiver<UnnamedMessage>,
    client: Client,
}

pub enum UnnamedMessage {
    MediaUpload {
        filename: String,
        bytes: Vec<u8>,
        response: oneshot::Sender<Media>,
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
        let handle = Handle { globals };
        let client = Client::new(token.clone().into()).with_handler(Box::new(handle));
        Self {
            client,
            recv,
        }
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
    match msg {
        UnnamedMessage::MediaUpload {
            filename,
            bytes,
            response,
        } => {
            let req = MediaCreate {
                filename,
                size: bytes.len() as u64,
                alt: None,
                url: None,
                source_url: None,
            };
            let upload = http.media_create(&req).await?;
            let media = http.media_upload(&upload, bytes).await?;
            let _ = response.send(media.expect("failed to upload media!"));
        }
        UnnamedMessage::MessageCreate {
            thread_id,
            req,
            response,
        } => {
            let res = http.message_create(thread_id, &req).await?;
            let _ = response.send(res);
        }
        UnnamedMessage::MessageUpdate {
            thread_id,
            message_id,
            req,
            response,
        } => {
            let res = http.message_update(thread_id, message_id, &req).await?;
            let _ = response.send(res);
        }
        UnnamedMessage::MessageDelete {
            thread_id,
            message_id,
            response,
        } => {
            http.message_delete(thread_id, message_id).await?;
            let _ = response.send(());
        }
        UnnamedMessage::MessageGet {
            thread_id,
            message_id,
            response,
        } => {
            let res = http.message_get(thread_id, message_id).await?;
            let _ = response.send(res);
        }
    }
    Ok(())
}
