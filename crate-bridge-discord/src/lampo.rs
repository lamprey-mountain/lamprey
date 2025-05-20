use std::sync::Arc;

use anyhow::{Error, Result};
use common::v1::types::{
    self, Media, MediaCreate, MediaCreateSource, MediaId, MessageCreate, MessageId, RoomId,
    Session, Thread, ThreadId, User, UserCreate, UserId,
};
use sdk::{Client, EventHandler, Http};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

use crate::{
    common::{Globals, GlobalsTrait},
    portal::PortalMessage,
};

pub struct Lampo {
    recv: mpsc::Receiver<LampoMessage>,
    client: Client,
}

pub enum LampoMessage {
    Handle {
        response: oneshot::Sender<LampoHandle>,
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
        self.globals.portal_send(
            message.thread_id,
            PortalMessage::LampoMessageUpsert { message },
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

impl Lampo {
    pub fn new(globals: Arc<Globals>, recv: mpsc::Receiver<LampoMessage>) -> Self {
        let token = std::env::var("MY_TOKEN").expect("missing MY_TOKEN");
        let base_url = std::env::var("BASE_URL").expect("missing BASE_URL");
        let base_url_ws = std::env::var("BASE_URL_WS").expect("missing BASE_URL_WS");
        let handle = Handle { globals };
        let mut client = Client::new(token.clone().into()).with_handler(Box::new(handle));
        client.http = client.http.with_base_url(base_url.parse().unwrap());
        client.syncer = client.syncer.with_base_url(base_url_ws.parse().unwrap());
        Self { client, recv }
    }

    pub async fn connect(mut self) -> Result<()> {
        tokio::spawn(async move {
            while let Some(msg) = self.recv.recv().await {
                match handle(msg, &self.client.http).await {
                    Ok(_) => {}
                    Err(err) => error!("{err}"),
                };
            }
        });

        let _ = self.client.syncer.connect().await;
        Ok(())
    }
}

async fn handle(msg: LampoMessage, http: &Http) -> Result<()> {
    match msg {
        LampoMessage::Handle { response } => {
            let _ = response.send(LampoHandle { http: http.clone() });
        }
    }
    Ok(())
}

pub struct LampoHandle {
    http: Http,
}

impl LampoHandle {
    pub async fn media_upload(
        &self,
        filename: String,
        bytes: Vec<u8>,
        user_id: UserId,
    ) -> Result<Media> {
        let req = MediaCreate {
            alt: None,
            source: MediaCreateSource::Upload {
                filename,
                size: bytes.len() as u64,
            },
        };
        let upload = self.http.for_puppet(user_id).media_create(&req).await?;
        let media = self
            .http
            .for_puppet(user_id)
            .media_upload(&upload, bytes)
            .await?;
        media.ok_or(anyhow::anyhow!("failed to upload"))
    }

    pub async fn media_info(&self, media_id: MediaId) -> Result<Media> {
        let media = self.http.media_info_get(media_id).await?;
        Ok(media)
    }

    pub async fn message_get(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
    ) -> Result<types::Message> {
        let res = self.http.message_get(thread_id, message_id).await?;
        Ok(res)
    }

    pub async fn message_create(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        req: MessageCreate,
    ) -> Result<types::Message> {
        let res = self
            .http
            .for_puppet(user_id)
            .message_create(thread_id, &req)
            .await?;
        Ok(res)
    }

    pub async fn message_update(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        user_id: UserId,
        req: types::MessagePatch,
    ) -> Result<types::Message> {
        let res = self
            .http
            .for_puppet(user_id)
            .message_update(thread_id, message_id, &req)
            .await?;
        Ok(res)
    }

    pub async fn message_delete(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        user_id: UserId,
    ) -> Result<()> {
        self.http
            .for_puppet(user_id)
            .message_delete(thread_id, message_id)
            .await?;
        Ok(())
    }

    pub async fn puppet_ensure(&self, name: String, key: String, room_id: RoomId) -> Result<User> {
        let user = self
            .http
            .user_create(&UserCreate {
                name,
                description: None,
                user_type: types::UserType::Puppet {
                    owner_id: "01943cc1-62e0-7c0e-bb9b-a4ff42864d69".parse().unwrap(),
                    external_platform: types::ExternalPlatform::Discord,
                    external_id: key,
                    external_url: None,
                    alias_id: None,
                },
            })
            .await?;
        self.http.room_member_put(room_id, user.id).await?;
        Ok(user)
    }

    pub async fn user_fetch(&self, user_id: UserId) -> Result<User> {
        let res = self.http.user_get(user_id).await?;
        Ok(res)
    }
}
