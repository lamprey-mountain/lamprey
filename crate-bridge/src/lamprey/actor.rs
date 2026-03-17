//! Lamprey actor - handles communication with the Lamprey chat service

use std::sync::Arc;

use anyhow::Result;
use common::v1::types::{
    self,
    pagination::{PaginationQuery, PaginationResponse},
    presence, Channel, ChannelId, ChannelType, MessageCreate, MessageId, MessageSync, RoomId,
    Session, User, UserId,
};
use common::v2::types::message::Message as LMessage;
use common::{v1::types::util::Time, v2::types::media::Media};
use kameo::message::Context;
use kameo::prelude::*;
use sdk::{Client, Http};
use tokio::sync::broadcast;
use tracing::{debug, error, info};

use crate::bridge_common::Globals;
use crate::db::Data;
use crate::portal::{Portal, PortalMessage};

pub use crate::lamprey::messages::{LampreyMessage, LampreyResponse};

pub struct Lamprey {
    http: Http,
    globals: Arc<Globals>,
    media_processed_tx: broadcast::Sender<Media>,
}

impl kameo::Actor for Lamprey {
    type Args = Arc<Globals>;
    type Error = anyhow::Error;

    async fn on_start(
        globals: Self::Args,
        _actor_ref: kameo::prelude::ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let token = globals.config.lamprey_token.clone();
        let base_url = globals.config.lamprey_base_url.clone();
        let ws_url = globals.config.lamprey_ws_url.clone();
        let (media_processed_tx, _) = broadcast::channel::<Media>(1024);
        let handle = LampreyEventHandler {
            globals: globals.clone(),
            media_processed_tx: media_processed_tx.clone(),
        };
        let mut client = Client::new(token.clone().into()).with_handler(Box::new(handle));
        client.http = if let Some(base_url) = base_url {
            client.http.with_base_url(base_url.parse().unwrap())
        } else {
            client.http
        };
        let mut syncer = if let Some(ws_url) = ws_url {
            client.syncer.with_base_url(ws_url.parse().unwrap())
        } else {
            client.syncer
        };

        tokio::spawn(async move {
            if let Err(e) = syncer.connect().await {
                tracing::error!("lamprey syncer error: {e}");
            }
        });

        Ok(Self {
            http: client.http,
            globals,
            media_processed_tx,
        })
    }
}

impl Message<LampreyMessage> for Lamprey {
    type Reply = Result<LampreyResponse>;

    async fn handle(
        &mut self,
        msg: LampreyMessage,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let res = crate::lamprey::handlers::handle_lamprey_message(
            &self.http,
            self.globals.clone(),
            self.media_processed_tx.clone(),
            msg,
        )
        .await;

        if let Err(e) = res {
            error!("lamprey actor handler failed: {:?}", e);
            return Err(e);
        }

        res
    }
}

pub struct LampreyHandle {
    pub lamprey_ref: ActorRef<Lamprey>,
    pub globals: Arc<Globals>,
}

impl LampreyHandle {
    pub async fn media_upload(
        &self,
        filename: String,
        bytes: Vec<u8>,
        user_id: UserId,
    ) -> Result<Media> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::MediaUpload {
                filename,
                bytes,
                user_id,
            })
            .await?;
        match response {
            LampreyResponse::Media(media) => Ok(media),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn message_get(
        &self,
        thread_id: ChannelId,
        message_id: MessageId,
    ) -> Result<LMessage> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::MessageGet {
                thread_id,
                message_id,
            })
            .await?;
        match response {
            LampreyResponse::Message(msg) => Ok(msg),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn message_list(
        &self,
        thread_id: ChannelId,
        query: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<LMessage>> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::MessageList {
                thread_id,
                query: Arc::new(query),
            })
            .await?;
        match response {
            LampreyResponse::MessageList(page) => Ok(page),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn message_create(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
        req: MessageCreate,
    ) -> Result<LMessage> {
        self.message_create_with_timestamp(thread_id, user_id, req, Time::now_utc())
            .await
    }

    pub async fn message_create_with_timestamp(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
        req: MessageCreate,
        timestamp: Time,
    ) -> Result<LMessage> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::MessageCreateWithTimestamp {
                thread_id,
                user_id,
                req,
                timestamp,
            })
            .await?;
        match response {
            LampreyResponse::Message(msg) => Ok(msg),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn message_update(
        &self,
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        req: types::MessagePatch,
    ) -> Result<LMessage> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::MessageUpdate {
                thread_id,
                message_id,
                user_id,
                req,
            })
            .await?;
        match response {
            LampreyResponse::Message(msg) => Ok(msg),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn message_delete(
        &self,
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
    ) -> Result<()> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::MessageDelete {
                thread_id,
                message_id,
                user_id,
            })
            .await?;
        match response {
            LampreyResponse::Empty => Ok(()),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn message_react(
        &self,
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        reaction: String,
    ) -> Result<()> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::MessageReact {
                thread_id,
                message_id,
                user_id,
                reaction,
            })
            .await?;
        match response {
            LampreyResponse::Empty => Ok(()),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn message_unreact(
        &self,
        thread_id: ChannelId,
        message_id: MessageId,
        user_id: UserId,
        reaction: String,
    ) -> Result<()> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::MessageUnreact {
                thread_id,
                message_id,
                user_id,
                reaction,
            })
            .await?;
        match response {
            LampreyResponse::Empty => Ok(()),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn typing_start(&self, thread_id: ChannelId, user_id: UserId) -> Result<()> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::TypingStart { thread_id, user_id })
            .await?;
        match response {
            LampreyResponse::Empty => Ok(()),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn puppet_ensure(
        &self,
        name: String,
        key: String,
        room_id: RoomId,
        bot: bool,
    ) -> Result<User> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::PuppetEnsure {
                name,
                key,
                room_id,
                bot,
            })
            .await?;
        match response {
            LampreyResponse::User(user) => Ok(user),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn user_fetch(&self, user_id: UserId) -> Result<User> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::UserFetch { user_id })
            .await?;
        match response {
            LampreyResponse::User(user) => Ok(user),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn user_update(&self, user_id: UserId, patch: types::UserPatch) -> Result<User> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::UserUpdate { user_id, patch })
            .await?;
        match response {
            LampreyResponse::User(user) => Ok(user),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn user_set_presence(
        &self,
        user_id: UserId,
        patch: presence::Presence,
    ) -> Result<()> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::UserSetPresence { user_id, patch })
            .await?;
        match response {
            LampreyResponse::Empty => Ok(()),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn room_member_patch(
        &self,
        room_id: RoomId,
        user_id: UserId,
        patch: types::RoomMemberPatch,
    ) -> Result<types::RoomMember> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::RoomMemberPatch {
                room_id,
                user_id,
                patch,
            })
            .await?;
        match response {
            LampreyResponse::RoomMember(member) => Ok(member),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn room_threads(&self, room_id: RoomId) -> Result<Vec<Channel>> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::RoomThreads { room_id })
            .await?;
        match response {
            LampreyResponse::RoomThreads(threads) => Ok(threads),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }

    pub async fn create_thread(
        &self,
        room_id: RoomId,
        name: String,
        topic: Option<String>,
        ty: ChannelType,
        parent_id: Option<ChannelId>,
    ) -> Result<Channel> {
        let response = self
            .lamprey_ref
            .ask(LampreyMessage::CreateThread {
                room_id,
                name,
                topic,
                ty,
                parent_id,
            })
            .await?;
        match response {
            LampreyResponse::Channel(channel) => Ok(channel),
            _ => Err(anyhow::anyhow!("unexpected response type")),
        }
    }
}

pub struct LampreyEventHandler {
    pub globals: Arc<Globals>,
    pub media_processed_tx: broadcast::Sender<Media>,
}

#[async_trait::async_trait]
impl sdk::EventHandler for LampreyEventHandler {
    type Error = anyhow::Error;

    fn ready(
        &mut self,
        user: Option<User>,
        _session: Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async move {
            if let Some(user) = user {
                self.globals.set_lamprey_user_id(user.id)?;
                info!("lamprey ready, user id: {}", user.id);
            }
            Ok(())
        }
    }

    fn error(
        &mut self,
        err: String,
        code: Option<common::v1::types::error::SyncError>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async move {
            if let Some(code) = code {
                tracing::error!("lamprey sync error [{code:?}]: {err}");
            } else {
                tracing::error!("lamprey sync error: {err}");
            }
            Ok(())
        }
    }

    fn sync(
        &mut self,
        msg: MessageSync,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async move { self.handle_sync(msg).await }
    }
}

impl LampreyEventHandler {
    async fn handle_sync(&self, msg: MessageSync) -> Result<()> {
        debug!("got lamprey sync {msg:?}");
        match msg {
            MessageSync::MessageCreate { message } => {
                let Ok(Some(config)) = self
                    .globals
                    .get_portal_by_thread_id(message.channel_id)
                    .await
                else {
                    return Ok(());
                };
                let portal_ref = self
                    .globals
                    .portals
                    .entry(config.lamprey_thread_id)
                    .or_insert_with(|| Portal::spawn((self.globals.clone(), config.to_owned())));
                let _ = portal_ref
                    .tell(PortalMessage::LampreyMessageCreate { message })
                    .await;
            }
            MessageSync::MessageUpdate { message } => {
                let Ok(Some(config)) = self
                    .globals
                    .get_portal_by_thread_id(message.channel_id)
                    .await
                else {
                    return Ok(());
                };
                let portal_ref = self
                    .globals
                    .portals
                    .entry(config.lamprey_thread_id)
                    .or_insert_with(|| Portal::spawn((self.globals.clone(), config.to_owned())));
                let _ = portal_ref
                    .tell(PortalMessage::LampreyMessageUpdate { message })
                    .await;
            }
            MessageSync::MessageDelete {
                channel_id,
                message_id,
            } => {
                let Ok(Some(config)) = self.globals.get_portal_by_thread_id(channel_id).await
                else {
                    return Ok(());
                };
                let portal_ref = self
                    .globals
                    .portals
                    .entry(config.lamprey_thread_id)
                    .or_insert_with(|| Portal::spawn((self.globals.clone(), config.to_owned())));
                let _ = portal_ref
                    .tell(PortalMessage::LampreyMessageDelete { message_id })
                    .await;
            }
            MessageSync::MediaProcessed { media, .. } => {
                let _ = self.media_processed_tx.send(media);
            }
            _ => {} // Other sync messages are ignored
        }
        Ok(())
    }
}
