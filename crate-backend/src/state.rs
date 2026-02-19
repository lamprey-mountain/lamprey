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
use futures::{Stream, StreamExt};
use lamprey_backend_core::Error;
use serde::{Deserialize, Serialize};
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

#[cfg(any())]
mod queue;

type BoxStream<T> = std::pin::Pin<Box<dyn Stream<Item = T> + Send>>;

/// Internal broadcast envelope containing a message and optional nonce
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBroadcastInner {
    pub message: MessageSync,
    pub nonce: Option<String>,
}

pub struct ServerStateInner {
    pub tokio: TokioHandle,
    pub config: Config,
    pub pool: PgPool,
    pub services: Weak<Services>,
    pub blobs: opendal::Operator, // TODO: write a wrapper around this?
    pub messaging: MessagingService,
}

pub enum MessagingService {
    /// use tokio channels to broadcast events
    Memory {
        /// ALL events on the server
        sushi: Sender<MessageBroadcastInner>,

        /// ALL events for voice sfus
        sushi_sfu: Sender<SfuCommand>,
    },

    /// use nats to broadcast events
    Nats(async_nats::Client),
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
        self.broadcast_inner(MessageBroadcastInner {
            message: msg,
            nonce: None,
        })
    }

    /// emit a message to everyone in a channel
    pub async fn broadcast_channel(
        &self,
        _thread_id: ChannelId,
        _user_id: UserId, // TODO: remove
        msg: MessageSync,
    ) -> Result<()> {
        self.broadcast_inner(MessageBroadcastInner {
            message: msg,
            nonce: None,
        })
    }

    /// emit a message to everyone in a channel with a nonce
    pub async fn broadcast_channel_with_nonce(
        &self,
        _thread_id: ChannelId,
        _user_id: UserId, // TODO: remove
        nonce: Option<&str>,
        msg: MessageSync,
    ) -> Result<()> {
        self.broadcast_inner(MessageBroadcastInner {
            message: msg,
            nonce: nonce.map(|s| s.to_string()),
        })
    }

    /// emit a message to a user
    pub async fn broadcast_user(&self, _user_id: UserId, msg: MessageSync) -> Result<()> {
        self.broadcast_inner(MessageBroadcastInner {
            message: msg,
            nonce: None,
        })
    }

    /// emit a message to everyone
    pub fn broadcast(&self, msg: MessageSync) -> Result<()> {
        self.broadcast_inner(MessageBroadcastInner {
            message: msg,
            nonce: None,
        })
    }

    /// emit a message to everyone
    fn broadcast_inner(&self, msg: MessageBroadcastInner) -> Result<()> {
        match &self.messaging {
            MessagingService::Memory { sushi, .. } => {
                let _ = sushi.send(msg);
            }
            MessagingService::Nats(client) => {
                let bytes = serde_json::to_vec(&msg)?;
                let _ = client.publish("sushi", bytes.into());
            }
        }
        Ok(())
    }

    /// emit a sfu command to everyone
    pub fn broadcast_sfu(&self, cmd: SfuCommand) -> Result<()> {
        match &self.messaging {
            MessagingService::Memory { sushi_sfu, .. } => {
                let _ = sushi_sfu.send(cmd);
            }
            MessagingService::Nats(client) => {
                let bytes = serde_json::to_vec(&cmd)?;
                let _ = client.publish("sushi_sfu", bytes.into());
            }
        }
        Ok(())
    }

    pub async fn subscribe_sushi(&self) -> Result<BoxStream<MessageBroadcastInner>> {
        match &self.messaging {
            MessagingService::Memory { sushi, .. } => {
                let stream = tokio_stream::wrappers::BroadcastStream::new(sushi.subscribe());
                Ok(Box::pin(stream.filter_map(|res| async move { res.ok() })))
            }
            MessagingService::Nats(client) => {
                let client = client.clone();
                let sub = client
                    .subscribe("sushi")
                    .await
                    .map_err(|e| Error::Internal(format!("NATS subscribe failed: {}", e)))?;
                let stream = futures::stream::unfold(sub, move |mut sub| async move {
                    match sub.next().await {
                        Some(msg) => match serde_json::from_slice(&msg.payload) {
                            Ok(inner) => Some((inner, sub)),
                            Err(e) => {
                                tracing::error!("NATS message deserialize failed: {}", e);
                                None
                            }
                        },
                        None => None,
                    }
                });
                Ok(Box::pin(stream))
            }
        }
    }

    pub async fn subscribe_sfu(&self) -> Result<BoxStream<SfuCommand>> {
        match &self.messaging {
            MessagingService::Memory { sushi_sfu, .. } => {
                let stream = tokio_stream::wrappers::BroadcastStream::new(sushi_sfu.subscribe());
                Ok(Box::pin(stream.filter_map(|res| async move { res.ok() })))
            }
            MessagingService::Nats(client) => {
                let client = client.clone();
                let sub = client
                    .subscribe("sushi_sfu")
                    .await
                    .map_err(|e| Error::Internal(format!("NATS subscribe failed: {}", e)))?;
                let stream = futures::stream::unfold(sub, move |mut sub| async move {
                    match sub.next().await {
                        Some(msg) => match serde_json::from_slice(&msg.payload) {
                            Ok(cmd) => Some((cmd, sub)),
                            Err(e) => {
                                tracing::error!("NATS message deserialize failed: {}", e);
                                None
                            }
                        },
                        None => None,
                    }
                });
                Ok(Box::pin(stream))
            }
        }
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
    pub async fn init(
        config: Config,
        pool: PgPool,
        blobs: opendal::Operator,
        nats: Option<async_nats::Client>,
    ) -> Self {
        // a bit hacky for now since i need to work around the existing ServerState
        // though i probably need some way to access global state/services from within them anyways
        let services = Arc::new_cyclic(|weak| {
            let inner = Arc::new(ServerStateInner {
                tokio: TokioHandle::current(),
                config,
                pool,
                services: weak.to_owned(),
                blobs,
                messaging: match nats {
                    Some(c) => MessagingService::Nats(c),
                    None => MessagingService::Memory {
                        // maybe i should increase the limit at some point? or make it unlimited?
                        sushi: tokio::sync::broadcast::channel(100).0,
                        sushi_sfu: tokio::sync::broadcast::channel(100).0,
                    },
                },
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
