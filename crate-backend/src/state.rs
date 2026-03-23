use std::{
    ops::Deref,
    sync::{Arc, Weak},
};

use common::v1::types::MessageSync;
use common::v2::types::message::Message;
use common::{
    v1::types::{voice::SfuCommand, AuditLogEntry, ChannelId, RoomId, UserId},
    v2::types::message::MessageVersion,
};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::{runtime::Handle as TokioHandle, sync::broadcast::Sender};
use tracing::{error, info};
use url::Url;

use crate::{
    config::{self, Config},
    data::{Data, Postgres},
    services::Services,
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
    // store the serve where this message came from
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
    Nats {
        client: async_nats::Client,

        /// ALL events on the server
        sushi: Sender<MessageBroadcastInner>,

        /// ALL events for voice sfus
        sushi_sfu: Sender<SfuCommand>,
    },
}

pub struct ServerState {
    pub inner: Arc<ServerStateInner>,
    pub services: Arc<Services>,
}

impl ServerStateInner {
    pub fn data(&self) -> Box<dyn Data> {
        Box::new(Postgres::new(self.pool.clone()))
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
        room_id: RoomId,
        user_id: UserId,
        msg: MessageSync,
    ) -> Result<()> {
        self.broadcast_room_with_nonce(room_id, user_id, None, msg)
            .await
    }

    /// emit a message to everyone in a room with a nonce
    pub async fn broadcast_room_with_nonce(
        &self,
        _room_id: RoomId,
        _user_id: UserId, // TODO: remove
        nonce: Option<&str>,
        msg: MessageSync,
    ) -> Result<()> {
        self.broadcast_inner(MessageBroadcastInner {
            message: msg,
            nonce: nonce.map(|s| s.to_string()),
        })
    }

    /// emit a message to everyone in a channel
    pub async fn broadcast_channel(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
        msg: MessageSync,
    ) -> Result<()> {
        self.broadcast_channel_with_nonce(thread_id, user_id, None, msg)
            .await
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
    pub async fn broadcast_user(&self, user_id: UserId, msg: MessageSync) -> Result<()> {
        self.broadcast_user_with_nonce(user_id, None, msg).await
    }

    /// emit a message to a user with a nonce
    pub async fn broadcast_user_with_nonce(
        &self,
        _user_id: UserId,
        nonce: Option<&str>,
        msg: MessageSync,
    ) -> Result<()> {
        self.broadcast_inner(MessageBroadcastInner {
            message: msg,
            nonce: nonce.map(|s| s.to_string()),
        })
    }

    /// emit a message to everyone
    pub fn broadcast(&self, msg: MessageSync) -> Result<()> {
        self.broadcast_with_nonce(None, msg)
    }

    /// emit a message to everyone with a nonce
    pub fn broadcast_with_nonce(&self, nonce: Option<&str>, msg: MessageSync) -> Result<()> {
        self.broadcast_inner(MessageBroadcastInner {
            message: msg,
            nonce: nonce.map(|s| s.to_string()),
        })
    }

    /// emit a message to everyone
    fn broadcast_inner(&self, msg: MessageBroadcastInner) -> Result<()> {
        match &self.messaging {
            MessagingService::Memory { sushi, .. } => {
                let _ = sushi.send(msg);
            }
            MessagingService::Nats { client, .. } => {
                let bytes = serde_json::to_vec(&msg)?;
                let client = client.clone();
                self.tokio.spawn(async move {
                    if let Err(e) = client.publish("sushi".to_string(), bytes.into()).await {
                        error!("NATS publish failed: {}", e);
                    }
                });
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
            MessagingService::Nats { client, .. } => {
                let bytes = serde_json::to_vec(&cmd)?;
                let client = client.clone();
                self.tokio.spawn(async move {
                    if let Err(e) = client.publish("sushi_sfu".to_string(), bytes.into()).await {
                        error!("NATS publish failed: {}", e);
                    }
                });
            }
        }
        Ok(())
    }

    pub async fn subscribe_sushi(&self) -> Result<BoxStream<MessageBroadcastInner>> {
        match &self.messaging {
            MessagingService::Memory { sushi, .. } | MessagingService::Nats { sushi, .. } => {
                let stream = tokio_stream::wrappers::BroadcastStream::new(sushi.subscribe());
                Ok(Box::pin(stream.filter_map(|res| async move { res.ok() })))
            }
        }
    }

    pub async fn subscribe_sfu(&self) -> Result<BoxStream<SfuCommand>> {
        match &self.messaging {
            MessagingService::Memory { sushi_sfu, .. }
            | MessagingService::Nats { sushi_sfu, .. } => {
                let stream = tokio_stream::wrappers::BroadcastStream::new(sushi_sfu.subscribe());
                Ok(Box::pin(stream.filter_map(|res| async move { res.ok() })))
            }
        }
    }

    pub fn get_s3_url(&self, path: &str) -> Result<Url> {
        let mut u = Url::parse("s3://")?;
        match &self.config.blobs {
            config::ConfigBlobs::S3(s3) => {
                u.set_host(Some(&s3.bucket))?;
            }
            config::ConfigBlobs::Fs(_) => {
                u.set_host(Some("localhost"))?;
            }
        }
        u.set_path(path);
        Ok(u)
    }

    /// presigns every relevant url in a piece of media
    pub async fn presign(&self, _media: &mut common::v2::types::media::Media) -> Result<()> {
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
            common::v2::types::message::MessageType::DefaultMarkdown(m) => {
                for attachment in &mut m.attachments {
                    let common::v2::types::message::MessageAttachmentType::Media { media } =
                        &mut attachment.ty;
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
                    Some(c) => {
                        info!("using NATS for messaging");
                        let (sushi_tx, _) = tokio::sync::broadcast::channel(100);
                        let (sushi_sfu_tx, _) = tokio::sync::broadcast::channel(100);

                        let c_clone = c.clone();
                        let sushi_tx_clone = sushi_tx.clone();
                        tokio::spawn(async move {
                            let mut sub = match c_clone.subscribe("sushi").await {
                                Ok(sub) => sub,
                                Err(e) => {
                                    error!("failed to subscribe to NATS 'sushi': {}", e);
                                    return;
                                }
                            };
                            while let Some(msg) = sub.next().await {
                                if let Ok(m) = serde_json::from_slice(&msg.payload) {
                                    let _ = sushi_tx_clone.send(m);
                                }
                            }
                        });

                        let c_clone = c.clone();
                        let sushi_sfu_tx_clone = sushi_sfu_tx.clone();
                        tokio::spawn(async move {
                            let mut sub = match c_clone.subscribe("sushi_sfu").await {
                                Ok(sub) => sub,
                                Err(e) => {
                                    error!("failed to subscribe to NATS 'sushi_sfu': {}", e);
                                    return;
                                }
                            };
                            while let Some(msg) = sub.next().await {
                                if let Ok(m) = serde_json::from_slice(&msg.payload) {
                                    let _ = sushi_sfu_tx_clone.send(m);
                                }
                            }
                        });

                        MessagingService::Nats {
                            client: c,
                            sushi: sushi_tx,
                            sushi_sfu: sushi_sfu_tx,
                        }
                    }
                    None => {
                        info!("using in-memory messaging");
                        MessagingService::Memory {
                            // maybe i should increase the limit at some point? or make it unlimited?
                            sushi: tokio::sync::broadcast::channel(100).0,
                            sushi_sfu: tokio::sync::broadcast::channel(100).0,
                        }
                    }
                },
            });
            Services::new(inner.clone())
        });
        services.start_background_tasks().await;
        Self {
            inner: services.state.clone(),
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
