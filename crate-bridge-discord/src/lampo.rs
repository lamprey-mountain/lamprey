use std::sync::Arc;

use anyhow::{Error, Result};
use common::v1::types::{
    self, Media, MediaCreate, MediaCreateSource, MediaId, MessageCreate, MessageId, RoomId,
    Session, Thread, ThreadId, User, UserCreate, UserId,
};
use sdk::{Client, EventHandler, Http};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};
use uuid::uuid;

use crate::{
    common::{Globals, GlobalsTrait},
    portal::PortalMessage,
};

pub struct Lampo {
    recv: mpsc::Receiver<LampoMessage>,
    client: Client,
}

pub enum LampoMessage {
    MediaUpload {
        filename: String,
        bytes: Vec<u8>,
        user_id: UserId,
        response: oneshot::Sender<Media>,
    },
    MediaInfo {
        media_id: MediaId,
        response: oneshot::Sender<Media>,
    },
    MessageGet {
        thread_id: ThreadId,
        message_id: MessageId,
        response: oneshot::Sender<types::Message>,
    },
    MessageCreate {
        thread_id: ThreadId,
        user_id: UserId,
        req: MessageCreate,
        response: oneshot::Sender<types::Message>,
    },
    MessageUpdate {
        thread_id: ThreadId,
        message_id: MessageId,
        user_id: UserId,
        req: types::MessagePatch,
        response: oneshot::Sender<types::Message>,
    },
    MessageDelete {
        thread_id: ThreadId,
        message_id: MessageId,
        user_id: UserId,
        response: oneshot::Sender<()>,
    },
    PuppetEnsure {
        name: String,
        key: String,
        room_id: RoomId,
        response: oneshot::Sender<User>,
    },
    UserFetch {
        user_id: UserId,
        response: oneshot::Sender<User>,
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
        if message.author_id == UserId::from(uuid!("01943cc1-62e0-7c0e-bb9b-a4ff42864d69")) {
            return Ok(());
        }
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
        LampoMessage::MediaUpload {
            filename,
            bytes,
            response,
            user_id,
        } => {
            let req = MediaCreate {
                alt: None,
                source: MediaCreateSource::Upload {
                    filename,
                    size: bytes.len() as u64,
                },
            };
            let upload = http.for_puppet(user_id).media_create(&req).await?;
            let media = http
                .for_puppet(user_id)
                .media_upload(&upload, bytes)
                .await?;
            let _ = response.send(media.expect("failed to upload media!"));
        }
        LampoMessage::MediaInfo { media_id, response } => {
            let media = http.media_info_get(media_id).await?;
            let _ = response.send(media);
        }
        LampoMessage::MessageCreate {
            thread_id,
            req,
            response,
            user_id,
        } => {
            let res = http
                .for_puppet(user_id)
                .message_create(thread_id, &req)
                .await?;
            let _ = response.send(res);
        }
        LampoMessage::MessageUpdate {
            thread_id,
            message_id,
            req,
            response,
            user_id,
        } => {
            let res = http
                .for_puppet(user_id)
                .message_update(thread_id, message_id, &req)
                .await?;
            let _ = response.send(res);
        }
        LampoMessage::MessageDelete {
            thread_id,
            message_id,
            response,
            user_id,
        } => {
            http.for_puppet(user_id)
                .message_delete(thread_id, message_id)
                .await?;
            let _ = response.send(());
        }
        LampoMessage::MessageGet {
            thread_id,
            message_id,
            response,
        } => {
            let res = http.message_get(thread_id, message_id).await?;
            let _ = response.send(res);
        }
        // UnnamedMessage::MessageGet {
        //     thread_id,
        //     message_id,
        //     response,
        // } => {
        //     let res = http.message_get(thread_id, message_id).await?;
        //     let _ = response.send(res);
        // }
        LampoMessage::UserFetch { user_id, response } => {
            let res = http.user_get(user_id).await?;
            let _ = response.send(res);
        }
        LampoMessage::PuppetEnsure {
            name,
            room_id,
            key,
            response,
        } => {
            let user = http
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
            http.room_member_put(room_id, user.id).await?;
            let _ = response.send(user);
        }
    }
    Ok(())
}
