use std::sync::Arc;

use anyhow::{Error, Result};
use common::v1::types::{
    self, misc::UserIdReq, pagination::PaginationQuery, ApplicationId, Media, MediaCreate,
    MediaCreateSource, MessageCreate, MessageId, MessageSync, RoomId, Session, Thread, ThreadId,
    ThreadType, User, UserId,
};
use sdk::{Client, EventHandler, Http};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};

use crate::{
    bridge::BridgeMessage,
    common::{Globals, GlobalsTrait},
    data::Data,
    portal::PortalMessage,
};

pub struct Lamprey {
    recv: mpsc::Receiver<LampreyMessage>,
    client: Client,
}

pub enum LampreyMessage {
    Handle {
        response: oneshot::Sender<LampreyHandle>,
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

    async fn sync(&mut self, msg: MessageSync) -> std::result::Result<(), Self::Error> {
        match msg {
            MessageSync::ThreadCreate { thread } => {
                info!("chat upsert thread");
                let Ok(realms) = self.globals.get_realms().await else {
                    return Ok(());
                };

                let Some(realm_config) = realms
                    .iter()
                    .find(|c| Some(c.lamprey_room_id) == thread.room_id)
                else {
                    return Ok(());
                };

                if !realm_config.continuous {
                    return Ok(());
                }

                if self
                    .globals
                    .get_portal_by_thread_id(thread.id)
                    .await?
                    .is_some()
                {
                    return Ok(());
                }

                if let Err(e) = self
                    .globals
                    .bridge_chan
                    .send(BridgeMessage::LampreyThreadCreate {
                        thread_id: thread.id,
                        room_id: realm_config.lamprey_room_id,
                        thread_name: thread.name,
                        discord_guild_id: realm_config.discord_guild_id,
                    })
                {
                    error!("failed to send lamprey thread create message: {e}");
                }
            }
            MessageSync::MessageCreate { message } => {
                info!("lamprey message create");
                self.globals
                    .portal_send(
                        message.thread_id,
                        PortalMessage::LampreyMessageCreate { message },
                    )
                    .await;
            }
            MessageSync::MessageUpdate { message } => {
                info!("lamprey message update");
                self.globals
                    .portal_send(
                        message.thread_id,
                        PortalMessage::LampreyMessageUpdate { message },
                    )
                    .await;
            }
            MessageSync::MessageDelete {
                room_id: _,
                thread_id,
                message_id,
            } => {
                info!("lamprey message delete");
                self.globals
                    .portal_send(
                        thread_id,
                        PortalMessage::LampreyMessageDelete { message_id },
                    )
                    .await;
            }
            _ => {}
        }
        Ok(())
    }
}

impl Lamprey {
    pub fn new(globals: Arc<Globals>, recv: mpsc::Receiver<LampreyMessage>) -> Self {
        let token = globals.config.lamprey_token.clone();
        let base_url = globals.config.lamprey_base_url.clone();
        let ws_url = globals.config.lamprey_ws_url.clone();
        let handle = Handle { globals };
        let mut client = Client::new(token.clone().into()).with_handler(Box::new(handle));
        client.http = if let Some(base_url) = base_url {
            client.http.with_base_url(base_url.parse().unwrap())
        } else {
            client.http
        };
        client.syncer = if let Some(ws_url) = ws_url {
            client.syncer.with_base_url(ws_url.parse().unwrap())
        } else {
            client.syncer
        };
        Self { client, recv }
    }

    pub async fn connect(mut self) -> Result<()> {
        tokio::spawn(async move {
            while let Some(msg) = self.recv.recv().await {
                info!("got msg");
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

async fn handle(msg: LampreyMessage, http: &Http) -> Result<()> {
    match msg {
        LampreyMessage::Handle { response } => {
            let _ = response.send(LampreyHandle { http: http.clone() });
        }
    }
    Ok(())
}

pub struct LampreyHandle {
    http: Http,
}

impl LampreyHandle {
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

    pub async fn message_react(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        user_id: UserId,
        reaction: String,
    ) -> Result<()> {
        self.http
            .for_puppet(user_id)
            .message_react(thread_id, message_id, reaction)
            .await?;
        Ok(())
    }

    pub async fn message_unreact(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        user_id: UserId,
        reaction: String,
    ) -> Result<()> {
        self.http
            .for_puppet(user_id)
            .message_unreact(thread_id, message_id, reaction)
            .await?;
        Ok(())
    }

    pub async fn typing_start(&self, thread_id: ThreadId, user_id: UserId) -> Result<()> {
        self.http
            .for_puppet(user_id)
            .typing_start(thread_id)
            .await?;
        Ok(())
    }

    pub async fn puppet_ensure(
        &self,
        name: String,
        key: String,
        room_id: RoomId,
        bot: bool,
    ) -> Result<User> {
        let app_id: ApplicationId = "01943cc1-62e0-7c0e-bb9b-a4ff42864d69".parse().unwrap();
        let user = self
            .http
            .puppet_ensure(
                app_id,
                key,
                &types::PuppetCreate {
                    name,
                    description: None,
                    bot,
                    system: false,
                },
            )
            .await?;
        debug!("ensured user");
        self.http.room_member_put(room_id, user.id).await?;
        debug!("ensured room member");
        Ok(user)
    }

    pub async fn user_fetch(&self, user_id: UserId) -> Result<User> {
        let res = self.http.user_get(user_id).await?;
        Ok(res)
    }

    pub async fn user_update(&self, user_id: UserId, patch: &types::UserPatch) -> Result<User> {
        let res = self
            .http
            .for_puppet(user_id)
            .user_update(UserIdReq::UserId(user_id), patch)
            .await?;
        Ok(res)
    }

    pub async fn room_threads(&self, room_id: RoomId) -> Result<Vec<Thread>> {
        let mut all_threads = Vec::new();
        let mut query = PaginationQuery::default();
        loop {
            info!("get room threads");
            let res = self.http.thread_list(room_id, &query).await?;
            debug!("threads: {res:?}");
            all_threads.extend(res.items);
            if let Some(cursor) = res.cursor {
                query.from = Some(cursor.parse().unwrap());
            } else {
                break;
            }
            if !res.has_more {
                break;
            }
        }
        Ok(all_threads)
    }

    pub async fn create_thread(
        &self,
        room_id: RoomId,
        name: String,
        topic: Option<String>,
    ) -> Result<Thread> {
        let res = self
            .http
            .thread_create(
                room_id,
                &types::ThreadCreate {
                    name,
                    description: topic,
                    ty: ThreadType::Chat,
                    tags: None,
                    nsfw: false,
                },
            )
            .await?;
        Ok(res)
    }
}
