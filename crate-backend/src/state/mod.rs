//! global server state

use std::{
    ops::Deref,
    sync::{Arc, Weak},
    time::Duration,
};

use axum::extract::FromRef;
use common::v1::types::MessageSync;
use common::v1::types::{AuditLogEntry, ChannelId, RoomId, UserId, voice::messages::SfuCommand};
use futures::{Stream, StreamExt};
use lamprey_backend_data_postgres::{
    Data, Postgres,
    data::{Data2, postgres::PostgresPool},
};
use opendal::layers::LoggingLayer;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::runtime::Handle as TokioHandle;
use tracing::{info, warn};
use url::Url;

use crate::{
    config::{self, Config},
    services::Services,
    state::messaging::{Broadcast, Messaging},
};
use crate::{prelude::*, state::messaging::Transport};

#[cfg(any())]
mod queue;

mod messaging;

type BoxStream<T> = std::pin::Pin<Box<dyn Stream<Item = T> + Send>>;

/// Internal broadcast envelope containing a message and optional nonce
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBroadcastInner {
    pub message: MessageSync,
    pub nonce: Option<String>,
    // store the server where this message came from
}

// TODO: write a wrapper around blobs and jetstream instead of accessing them directly?
pub struct ServerStateInner {
    pub tokio: TokioHandle,
    pub config: Config,
    // pub database: Box<dyn Data2>, // TEMP
    pub database: Box<PostgresPool>,
    pub services: Weak<Services>,
    pub blobs: opendal::Operator,
    pub jetstream: Option<async_nats::jetstream::Context>,

    // the new server state
    pub new_state: ServerState2,
}

pub struct ServerState {
    pub inner: Arc<ServerStateInner>,
    pub services: Arc<Services>,
}

impl ServerStateInner {
    /// legacy: acquire a connection to the database that auto-commits on every query
    // TODO: remove
    pub fn data(&self) -> Box<dyn Data> {
        Box::new(Postgres {
            pool: self.database.pool.clone(),
            txn: None,
            use_legacy_behavior: true,
        })
    }

    pub fn database(&self) -> Box<dyn Data2<DataTxn = Postgres>> {
        Box::new((*self.database).clone())
    }

    /// acquire a transaction
    pub async fn acquire_data(&self) -> Result<Box<dyn Data>> {
        let txn_wrapped = self.database.begin().await?;
        Ok(Box::new(txn_wrapped))
    }

    pub fn services(&self) -> Arc<Services> {
        self.services
            .upgrade()
            .expect("services should always exist while serverstateinner is alive")
    }

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

    // TODO: use this instead
    /// emit a message to everyone in a room
    pub async fn broadcast_room2(&self, room_id: RoomId, msg: MessageSync) -> Result<()> {
        self.broadcast_room_with_nonce2(room_id, None, msg).await
    }

    // TODO: use this instead
    /// emit a message to everyone in a room with a nonce
    pub async fn broadcast_room_with_nonce2(
        &self,
        _room_id: RoomId,
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
        let _ = self.new_state.messaging().broadcast_room(
            RoomId::default(), // this is ignored
            MessageSync::from(msg.message),
        );
        Ok(())
    }

    /// emit a sfu command to everyone
    pub fn broadcast_sfu(&self, cmd: SfuCommand) -> Result<()> {
        let _ = self.new_state.messaging().broadcast_global(cmd);
        Ok(())
    }

    pub async fn subscribe_sushi(&self) -> Result<BoxStream<MessageBroadcastInner>> {
        let stream = self.new_state.messaging().subscribe().await?;
        Ok(Box::pin(stream.filter_map(|msg| async move {
            match msg {
                Broadcast::Sync(s) => Some(MessageBroadcastInner {
                    message: s.message,
                    nonce: s.nonce,
                }),
                _ => None,
            }
        })))
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

    /// get a handle to the new server state
    pub fn ss2(&self) -> ServerState2 {
        self.new_state.clone()
    }
}

impl ServerState {
    pub async fn init(
        config: Config,
        pool: PgPool,
        blobs: opendal::Operator,
        nats: Option<async_nats::Client>,
    ) -> Self {
        let state =
            ServerState2::legacy_init(config.clone(), pool.clone(), blobs.clone(), nats.clone())
                .await
                .expect("TODO better error handling");

        let inner = ServerStateInner {
            tokio: TokioHandle::current(),
            config,
            database: state.inner.database.clone(),
            services: Arc::downgrade(&state.inner.services),
            blobs,
            jetstream: nats.clone().map(async_nats::jetstream::new),
            new_state: state.clone(),
        };

        Self {
            inner: Arc::new(inner),
            services: state.services(),
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

// ===== NEW TYPES =====
// TODO: switch over to ServerState2 entirely
// looks like this type ended up being pretty similar to the original ServerState, oh well
// at least the fields won't be pub now?

/// global state for the server
#[derive(Clone)]
pub struct ServerState2 {
    inner: Arc<ServerStateInner2>,
}

struct ServerStateInner2 {
    /// config for this server
    config: Config,

    /// reference to the database for persistent data
    // database: Box<dyn Data2>,
    database: Box<PostgresPool>,

    /// storage for large blobs
    blobs: opendal::Operator,

    /// send and receive messages
    messaging: Messaging,

    /// services
    // hold a strong reference to Services so it isnt immediately dropped
    // yes, this does technically cause a memory leak. i'm not sure what the best way to fix this is though.
    services: Arc<Services>,
}

impl ServerState2 {
    pub async fn init_from_config(config: Config) -> Result<Self> {
        // lint config
        if config.http.contact.is_none() {
            warn!(
                "http.contact is not set in your config! set it so an email or something so webmasters can contact you."
            );
        }

        // setup the database connection
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&config.database_url)
            .await?;
        let database = Box::new(PostgresPool::new(pool));
        database.migrate().await?;

        // setup the object storage connection
        let blobs = match &config.blobs {
            config::ConfigBlobs::S3(s3) => {
                let builder = opendal::services::S3::default()
                    .bucket(&s3.bucket)
                    .endpoint(s3.endpoint.as_str())
                    .region(&s3.region)
                    .access_key_id(&s3.access_key_id)
                    .secret_access_key(s3.secret_access_key.load()?.as_ref());
                opendal::Operator::new(builder)?
                    .layer(LoggingLayer::default())
                    .finish()
            }
            config::ConfigBlobs::Fs(fs) => {
                let builder = opendal::services::Fs::default().root(fs.data_dir.to_str().unwrap());
                opendal::Operator::new(builder)?
                    .layer(LoggingLayer::default())
                    .finish()
            }
        };
        // TODO: don't require blobs to be healthy to start server
        blobs.check().await?;

        // set up messaging
        let transport = if let Some(nats_config) = &config.nats {
            info!("using NATS for messaging");
            let mut nats_options = async_nats::ConnectOptions::new();
            if let Some(credentials_path) = &nats_config.credentials {
                nats_options = nats_options
                    .credentials_file(credentials_path)
                    .await
                    .map_err(|e| Error::Internal(format!("NATS credentials file failed: {}", e)))?;
            }
            let nats = async_nats::connect_with_options(&nats_config.addr, nats_options)
                .await
                .map_err(|e| Error::Internal(format!("NATS connect failed: {}", e)))?;
            Transport::nats(nats)
        } else {
            info!("using in-memory messaging");
            Transport::memory()
        };
        let messaging = Messaging::new(transport);

        // create services and tie up the arc cycle
        let services = Arc::new_cyclic(|weak_services| {
            let state = ServerState2 {
                inner: Arc::new(ServerStateInner2 {
                    config,
                    services: weak_services.upgrade().unwrap(),
                    database,
                    blobs,
                    messaging,
                }),
            };

            Services::new(state)
        });

        services.start_background_tasks().await;

        // initialize server
        // TODO: setup_vapid_keys(&state).await?;
        // TODO: setup_server_room(&state).await?;

        Ok(services.state.clone())
    }

    // TEMP
    pub async fn legacy_init(
        config: Config,
        pool: PgPool,
        blobs: opendal::Operator,
        nats: Option<async_nats::Client>,
    ) -> Result<Self> {
        if config.http.contact.is_none() {
            warn!(
                "http.contact is not set in your config! set it so an email or something so webmasters can contact you."
            );
        }

        let database = Box::new(PostgresPool::new(pool));

        let transport = if let Some(nats) = nats {
            info!("using NATS for messaging");
            Transport::nats(nats)
        } else {
            info!("using in-memory messaging");
            Transport::memory()
        };
        let messaging = Messaging::new(transport);

        let services = Arc::new_cyclic(|weak_services| {
            let state = ServerState2 {
                inner: Arc::new(ServerStateInner2 {
                    config,
                    services: weak_services.upgrade().unwrap(),
                    database,
                    blobs,
                    messaging,
                }),
            };

            Services::new(state)
        });

        Ok(services.state.clone())
    }

    // TODO: maybe merge Server here...? maybe not...
    // /// start the http server
    // pub async fn serve(&self) -> Result<()> {
    //     todo!()
    // }

    /// legacy: acquire a connection to the database that auto-commits on every query
    // TEMP: compat
    pub fn data(&self) -> Box<dyn Data> {
        Box::new(Postgres {
            pool: self.inner.database.pool.clone(),
            txn: None,
            use_legacy_behavior: true,
        })
    }

    /// acquire/begin a database transaction
    pub async fn acquire(&self) -> Result<Box<dyn Data>> {
        let txn = self.inner.database.begin().await?;
        Ok(Box::new(txn))
    }

    pub fn services(&self) -> Arc<Services> {
        self.inner.services.clone()
    }

    pub fn messaging(&self) -> &Messaging {
        &self.inner.messaging
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    // TEMP: i should write a wrapper for opendal
    pub fn blobs(&self) -> &opendal::Operator {
        &self.inner.blobs
    }

    /// create a handle to the old ServerStateInner struct
    pub fn ss1(&self) -> Arc<ServerStateInner> {
        let inner = ServerStateInner {
            tokio: TokioHandle::current(),
            config: self.config().clone(),
            database: self.inner.database.clone(),
            services: Arc::downgrade(&self.inner.services),
            blobs: self.inner.blobs.clone(),
            jetstream: None, // FIXME: populate jetstream
            new_state: self.clone(),
        };
        Arc::new(inner)
    }
}

impl FromRef<Arc<ServerState>> for ServerState2 {
    fn from_ref(input: &Arc<ServerState>) -> Self {
        input.ss2()
    }
}
