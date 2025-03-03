use std::{
    ops::Deref,
    sync::{Arc, Weak},
    time::Duration,
};

use ::types::{Media, Message, RoomId, ThreadId, UrlEmbed, UserId};
use dashmap::DashMap;
use moka::future::Cache;
use sqlx::PgPool;
use tokio::sync::broadcast::Sender;
use types::{MessageSync, MessageType};
use url::Url;

use crate::{
    config::Config,
    data::{postgres::Postgres, Data},
    services::Services,
    sync::Connection,
    Result,
};

pub struct ServerStateInner {
    pub config: Config,
    pub pool: PgPool,
    pub services: Weak<Services>,

    // this is fine probably
    pub sushi: Sender<MessageSync>,
    // channel_user: Arc<DashMap<UserId, (Sender<MessageServer>, Receiver<MessageServer>)>>,

    // TODO: write a wrapper around this
    pub blobs: opendal::Operator,

    cache_presigned: Cache<Url, Url>,
}

// newly signed urls last for 24 hours = 1 day
const PRESIGNED_URL_LIFETIME: Duration = Duration::from_secs(60 * 60 * 24);

// the server will only return urls that are valid for at least 8 hours
const PRESIGNED_MIN_LIFETIME: Duration = Duration::from_secs(60 * 60 * 8);

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
        room_id: RoomId,
        user_id: UserId,
        reason: Option<String>,
        msg: MessageSync,
    ) -> Result<()> {
        if msg.is_room_audit_loggable() {
            self.data()
                .audit_logs_room_append(room_id, user_id, reason, msg.clone())
                .await?;
        }
        let _ = self.sushi.send(msg);
        Ok(())
    }

    pub async fn broadcast_thread(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        reason: Option<String>,
        msg: MessageSync,
    ) -> Result<()> {
        if msg.is_room_audit_loggable() {
            let thread = self
                .services()
                .threads
                .get(thread_id, Some(user_id))
                .await?;
            self.broadcast_room(thread.room_id, user_id, reason, msg)
                .await?;
        } else {
            let _ = self.sushi.send(msg);
        }
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
    pub async fn presign(&self, media: &mut Media) -> Result<()> {
        for t in media.all_tracks_mut() {
            t.url = self
                .cache_presigned
                .try_get_with(t.url.to_owned(), async {
                    let signed: Url = self
                        .blobs
                        .presign_read(t.url.path(), PRESIGNED_URL_LIFETIME)
                        .await?
                        .uri()
                        .to_string()
                        .parse()?;
                    crate::Result::Ok(signed)
                })
                .await
                .map_err(|err| err.fake_clone())?;
        }
        Ok(())
    }

    /// presigns every relevant url in a UrlEmbed
    pub async fn presign_url_embed(&self, embed: &mut UrlEmbed) -> Result<()> {
        if let Some(m) = &mut embed.media {
            self.presign(m).await?;
        }
        if let Some(m) = &mut embed.author_avatar {
            self.presign(m).await?;
        }
        if let Some(m) = &mut embed.site_avatar {
            self.presign(m).await?;
        }
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
                    self.presign_url_embed(emb).await?;
                }
            }
            _ => {}
        }
        Ok(())
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
                cache_presigned: Cache::builder()
                    .max_capacity(1_000_000)
                    .time_to_live(PRESIGNED_URL_LIFETIME - PRESIGNED_MIN_LIFETIME)
                    .build(),
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
