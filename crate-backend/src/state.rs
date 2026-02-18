use std::{
    ops::Deref,
    sync::{Arc, Weak},
};

use common::v1::types::{MessageSync, MessageType, RoomId};
use common::v2::types::message::Message;
use common::{
    v1::types::{voice::SfuCommand, AuditLogEntry, ChannelId, ConnectionId, Media, UserId},
    v2::types::message::MessageVersion,
};
use dashmap::DashMap;

use sqlx::PgPool;
use tokio::{runtime::Handle as TokioHandle, sync::broadcast::Sender};
use url::Url;

use crate::{
    config::Config,
    data::{postgres::Postgres, Data},
    services::Services,
    sync::Connection,
    Result,
};

pub struct ServerStateInner {
    pub tokio: TokioHandle,
    pub config: Config,
    pub pool: PgPool,
    pub services: Weak<Services>,

    // this is fine probably
    pub sushi: Sender<(MessageSync, Option<String>)>,
    // channel_user: Arc<DashMap<UserId, (Sender<MessageServer>, Receiver<MessageServer>)>>,
    pub sushi_sfu: Sender<SfuCommand>,

    // TODO: write a wrapper around this (media is kind of like this?)
    pub blobs: opendal::Operator,
}

pub struct ServerState {
    pub inner: Arc<ServerStateInner>,
    pub services: Arc<Services>,

    // TODO: limit number of connections per user, clean up old/unused entries
    pub syncers: Arc<DashMap<ConnectionId, Connection>>,
}

impl ServerStateInner {
    pub fn data(&self) -> Box<dyn Data> {
        Box::new(Postgres {
            pool: self.pool.clone(),
        })
    }

    pub fn services(&self) -> Arc<Services> {
        self.services
            .upgrade()
            .expect("services should always exist while serverstateinner is alive")
    }

    // fn acquire_data(&self) -> Box<dyn Data> {
    //     Box::new(Postgres {
    //         pool: self.pool.clone(),
    //     })
    // }

    /// emit a message to everyone in a room
    pub async fn broadcast_room(
        &self,
        _room_id: RoomId,
        _user_id: UserId, // TODO: remove
        msg: MessageSync,
    ) -> Result<()> {
        let _ = self.sushi.send((msg, None));
        Ok(())
    }

    /// emit a message to everyone in a channel
    // TODO: remove?
    pub async fn broadcast_channel(
        &self,
        _thread_id: ChannelId,
        _user_id: UserId, // TODO: remove
        msg: MessageSync,
    ) -> Result<()> {
        let _ = self.sushi.send((msg, None));
        Ok(())
    }

    pub async fn broadcast_channel2(
        &self,
        _channel_id: ChannelId,
        nonce: Option<&str>,
        msg: MessageSync,
    ) -> Result<()> {
        let _ = self.sushi.send((msg, nonce.map(|s| s.to_owned())));
        Ok(())
    }

    /// emit a message to a user
    pub async fn broadcast_user(&self, _user_id: UserId, msg: MessageSync) -> Result<()> {
        let _ = self.sushi.send((msg, None));
        Ok(())
    }

    /// emit a message to everyone
    pub fn broadcast(&self, msg: MessageSync) -> Result<()> {
        let _ = self.sushi.send((msg, None));
        Ok(())
    }

    pub fn get_s3_url(&self, path: &str) -> Result<Url> {
        let mut u = Url::parse("s3://")?;
        u.set_host(Some(&self.config.s3.bucket))?;
        u.set_path(path);
        Ok(u)
    }

    /// presigns every relevant url in a piece of media
    pub async fn presign(&self, _media: &mut Media) -> Result<()> {
        // in the past, media was served directly from s3
        // this doesn't do anything, but i'll keep it just in case
        Ok(())
    }

    pub async fn audit_log_append(&self, entry: AuditLogEntry) -> Result<()> {
        self.data().audit_logs_room_append(entry.clone()).await?;
        self.broadcast_room(
            entry.room_id,
            entry.user_id,
            MessageSync::AuditLogEntryCreate { entry },
        )
        .await?;
        Ok(())
    }

    /// presigns every relevant url in a Message
    pub async fn presign_message(&self, message: &mut Message) -> Result<()> {
        self.presign_message_version(&mut message.latest_version)
            .await
    }

    /// presigns every relevant url in a MessageVersion
    pub async fn presign_message_version(&self, ver: &mut MessageVersion) -> Result<()> {
        match &mut ver.message_type {
            MessageType::DefaultMarkdown(m) => {
                for media in &mut m.attachments {
                    self.presign(media).await?;
                }
                for emb in &mut m.embeds {
                    if let Some(m) = &mut emb.media {
                        self.presign(m).await?;
                    }
                    if let Some(m) = &mut emb.author_avatar {
                        self.presign(m).await?;
                    }
                    if let Some(m) = &mut emb.site_avatar {
                        self.presign(m).await?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl ServerState {
    pub async fn init(config: Config, pool: PgPool, blobs: opendal::Operator) -> Self {
        // a bit hacky for now since i need to work around the existing ServerState
        // though i probably need some way to access global state/services from within them anyways
        let services = Arc::new_cyclic(|weak| {
            let inner = Arc::new(ServerStateInner {
                tokio: TokioHandle::current(),
                config,
                pool,
                services: weak.to_owned(),
                blobs,

                // maybe i should increase the limit at some point? or make it unlimited?
                sushi: tokio::sync::broadcast::channel(100).0,
                sushi_sfu: tokio::sync::broadcast::channel(100).0,
            });
            Services::new(inner.clone())
        });
        services.start_background_tasks().await;
        Self {
            inner: services.state.clone(),
            syncers: Arc::new(DashMap::new()),
            // channel_user: Arc::new(DashMap::new()),
            services,
        }
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub fn data(&self) -> Box<dyn Data> {
        self.inner.data()
    }

    pub fn services(self: &Arc<Self>) -> Arc<Services> {
        self.services.clone()
    }
}

impl Deref for ServerState {
    type Target = ServerStateInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
