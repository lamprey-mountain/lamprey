use std::{
    ops::Deref,
    sync::{Arc, Weak},
};

use common::v1::types::{
    voice::SfuCommand, AuditLogEntry, Media, Message, RoomId, SfuId, ThreadId, UserId,
};
use common::v1::types::{MessageSync, MessageType};
use dashmap::DashMap;

use sqlx::PgPool;
use tokio::sync::broadcast::Sender;
use tracing::error;
use url::Url;

use crate::{
    config::Config,
    data::{postgres::Postgres, Data},
    services::Services,
    sync::Connection,
    Error, Result,
};

pub struct ServerStateInner {
    pub config: Config,
    pub pool: PgPool,
    pub services: Weak<Services>,

    // this is fine probably
    pub sushi: Sender<MessageSync>,
    // channel_user: Arc<DashMap<UserId, (Sender<MessageServer>, Receiver<MessageServer>)>>,
    pub sushi_sfu: Sender<SfuCommand>,

    // TODO: write a wrapper around this (media is kind of like this?)
    pub blobs: opendal::Operator,

    pub sfus: DashMap<SfuId, ()>,
    pub thread_to_sfu: DashMap<ThreadId, SfuId>,
}

pub struct ServerState {
    pub inner: Arc<ServerStateInner>,
    pub services: Arc<Services>,

    // TODO: limit number of connections per user, clean up old/unused entries
    pub syncers: Arc<DashMap<String, Connection>>,
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

    pub async fn broadcast_room(
        &self,
        _room_id: RoomId,
        _user_id: UserId, // TODO: remove
        msg: MessageSync,
    ) -> Result<()> {
        let _ = self.sushi.send(msg);
        Ok(())
    }

    pub async fn broadcast_thread(
        &self,
        _thread_id: ThreadId,
        _user_id: UserId,
        msg: MessageSync,
    ) -> Result<()> {
        let _ = self.sushi.send(msg);
        Ok(())
    }

    pub fn broadcast(&self, msg: MessageSync) -> Result<()> {
        let _ = self.sushi.send(msg);
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
        match &mut message.message_type {
            MessageType::DefaultMarkdown(message) => {
                for media in &mut message.attachments {
                    self.presign(media).await?;
                }
                for emb in &mut message.embeds {
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

    /// select the "best" sfu and pair it with this thread id. return the existing sfu id if it exists.
    ///
    /// currently "best" means the sfu with least load in terms of # of threads using it
    pub fn alloc_sfu(&self, thread_id: ThreadId) -> Result<SfuId> {
        if let Some(existing) = self.thread_to_sfu.get(&thread_id) {
            return Ok(*existing);
        }

        let sfu_thread_counts = DashMap::<SfuId, u64>::new();
        for i in &self.sfus {
            sfu_thread_counts.insert(*i.key(), 0);
        }
        for i in &self.thread_to_sfu {
            *sfu_thread_counts.get_mut(i.value()).unwrap() += 1;
        }
        let mut sorted: Vec<_> = sfu_thread_counts.into_iter().collect();
        sorted.sort_by_key(|(_, count)| *count);
        if let Some((chosen, _)) = sorted.first() {
            self.thread_to_sfu.insert(thread_id, *chosen);
            Ok(*chosen)
        } else {
            error!("no available sfu");
            Err(Error::BadStatic("no available sfu"))
        }
    }
}

impl ServerState {
    pub fn new(config: Config, pool: PgPool, blobs: opendal::Operator) -> Self {
        // a bit hacky for now since i need to work around the existing ServerState
        // though i probably need some way to access global state/services from within them anyways
        let services = Arc::new_cyclic(|weak| {
            let inner = Arc::new(ServerStateInner {
                config,
                pool,
                services: weak.to_owned(),
                blobs,

                // maybe i should increase the limit at some point? or make it unlimited?
                sushi: tokio::sync::broadcast::channel(100).0,
                sushi_sfu: tokio::sync::broadcast::channel(100).0,

                sfus: DashMap::new(),
                thread_to_sfu: DashMap::new(),
            });
            Services::new(inner.clone())
        });
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
