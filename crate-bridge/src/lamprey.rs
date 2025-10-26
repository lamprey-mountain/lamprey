use std::sync::Arc;

use anyhow::{Error, Result};
use common::v1::types::{
    self,
    misc::UserIdReq,
    pagination::{PaginationQuery, PaginationResponse},
    user_status, Channel, ChannelId, ChannelType, Media, MediaCreate, MediaCreateSource,
    MessageCreate, MessageId, MessageSync, RoomId, RoomMemberPut, Session, User, UserId,
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
    globals: Arc<Globals>,
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

    async fn error(&mut self, err: String) -> Result<()> {
        error!("lamprey sync error: {err}");
        Ok(())
    }

    async fn sync(&mut self, msg: MessageSync) -> Result<()> {
        match msg {
            MessageSync::ChannelCreate { channel: thread } => {
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
                        thread: *thread,
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
                        message.channel_id,
                        PortalMessage::LampreyMessageCreate { message },
                    )
                    .await;
            }
            MessageSync::MessageUpdate { message } => {
                info!("lamprey message update");
                self.globals
                    .portal_send(
                        message.channel_id,
                        PortalMessage::LampreyMessageUpdate { message },
                    )
                    .await;
            }
            MessageSync::MessageDelete {
                channel_id: thread_id,
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
        let handle = Handle {
            globals: globals.clone(),
        };
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
        Self {
            client,
            recv,
            globals,
        }
    }

    pub async fn connect(mut self) -> Result<()> {
        tokio::spawn(async move {
            while let Some(msg) = self.recv.recv().await {
                info!("got msg");
                match handle(self.globals.clone(), msg, &self.client.http).await {
                    Ok(_) => {}
                    Err(err) => error!("{err}"),
                };
            }
        });

        let _ = self.client.syncer.connect().await;
        Ok(())
    }
}

async fn handle(globals: Arc<Globals>, msg: LampreyMessage, http: &Http) -> Result<()> {
    match msg {
        LampreyMessage::Handle { response } => {
            let _ = response.send(LampreyHandle {
                globals,
                http: http.clone(),
            });
        }
    }
    Ok(())
}

pub struct LampreyHandle {
    http: Http,
    globals: Arc<Globals>,
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
        thread_id: ChannelId,
        message_id: MessageId,
    ) -> Result<types::Message> {
        let res = self.http.message_get(thread_id, message_id).await?;
        Ok(res)
    }

    pub async fn message_list(
        &self,
        thread_id: ChannelId,
        query: &PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<types::Message>> {
        let res = self.http.message_list(thread_id, query).await?;
        Ok(res)
    }

    pub async fn message_create(
        &self,
        thread_id: ChannelId,
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
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        req: types::MessagePatch,
    ) -> Result<types::Message> {
        let res = self
            .http
            .for_puppet(user_id)
            .message_edit(thread_id, message_id, &req)
            .await?;
        Ok(res)
    }

    pub async fn message_delete(
        &self,
        thread_id: ChannelId,
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
        thread_id: ChannelId,
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
        thread_id: ChannelId,
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

    pub async fn typing_start(&self, thread_id: ChannelId, user_id: UserId) -> Result<()> {
        self.http
            .for_puppet(user_id)
            .channel_typing(thread_id)
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
        let app_id = self.globals.config.lamprey_application_id;
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
        self.http
            .room_member_add(room_id, UserIdReq::UserId(user.id), &RoomMemberPut::default())
            .await?;
        debug!("ensured room member");
        Ok(user)
    }

    pub async fn user_fetch(&self, user_id: UserId) -> Result<User> {
        let res = self.http.user_get(UserIdReq::UserId(user_id)).await?;
        Ok(res.inner)
    }

    pub async fn user_update(&self, user_id: UserId, patch: &types::UserPatch) -> Result<User> {
        let res = self
            .http
            .for_puppet(user_id)
            .user_update(UserIdReq::UserId(user_id), patch)
            .await?;
        Ok(res)
    }

    pub async fn user_set_status(
        &self,
        user_id: UserId,
        patch: &user_status::StatusPatch,
    ) -> Result<()> {
        self.http
            .for_puppet(user_id)
            .user_set_status(UserIdReq::UserId(user_id), patch)
            .await?;
        Ok(())
    }

    pub async fn room_member_patch(
        &self,
        room_id: RoomId,
        user_id: UserId,
        patch: &types::RoomMemberPatch,
    ) -> Result<types::RoomMember> {
        let res = self
            .http
            .room_member_patch(room_id, UserIdReq::UserId(user_id), patch)
            .await?;
        Ok(res)
    }

    pub async fn room_threads(&self, room_id: RoomId) -> Result<Vec<Channel>> {
        let mut all_threads = Vec::new();
        let mut query = PaginationQuery::default();
        loop {
            info!("get room threads");
            let res = self.http.channel_list(room_id, &query).await?;
            debug!("threads: {res:?}");
            all_threads.extend(res.items);
            if let Some(cursor) = res.cursor {
                query.from = Some(cursor.parse()?);
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
        ty: ChannelType,
        parent_id: Option<ChannelId>,
    ) -> Result<Channel> {
        let res = self
            .http
            .channel_create_room(
                room_id,
                &types::ChannelCreate {
                    name,
                    description: topic,
                    ty,
                    tags: None,
                    nsfw: false,
                    recipients: None,
                    bitrate: None,
                    user_limit: None,
                    parent_id,
                    icon: None,
                },
            )
            .await?;
        Ok(res)
    }
}
