//! global server state

use std::{ops::Deref, sync::Weak};

use axum::extract::FromRef;
use common::v1::types::MessageSync;
use common::v1::types::{AuditLogEntry, ChannelId, RoomId, UserId, voice::messages::SfuCommand};
use futures::{Stream, StreamExt};
use lamprey_backend_data_postgres::{
    Postgres,
    data::{AnyData, Database, postgres::PostgresPool},
};
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle as TokioHandle;
use url::Url;

use crate::prelude::*;
use crate::state::messaging::BroadcastSync;
use crate::{
    config::{self, Config},
    services::Services,
    state::messaging::{Broadcast, Messaging},
};

#[cfg(any())]
mod queue;

pub mod messaging;

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
    pub messaging: Messaging,
    pub globals: Globals,
}

pub struct ServerState {
    pub inner: Arc<ServerStateInner>,
    pub services: Arc<Services>,
}

impl ServerStateInner {
    /// legacy: acquire a connection to the database that auto-commits on every query
    // TODO: remove
    pub fn data(&self) -> AnyData {
        Box::new(Postgres {
            pool: self.database.pool.clone(),
            txn: None,
            use_legacy_behavior: true,
        })
    }

    pub fn database(&self) -> Box<PostgresPool> {
        Box::new((*self.database).clone())
    }

    /// acquire a transaction
    pub async fn acquire_data(&self) -> Result<AnyData> {
        let txn_wrapped = self.database.begin().await?;
        Ok(txn_wrapped)
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
        let messaging = self.messaging.clone();
        let broadcast = BroadcastSync {
            message: msg.message,
            nonce: msg.nonce,
        };
        tokio::spawn(async move {
            let _ = messaging.broadcast_global(broadcast).await;
        });
        Ok(())
    }

    /// emit a sfu command to everyone
    pub fn broadcast_sfu(&self, cmd: SfuCommand) -> Result<()> {
        let _ = self.messaging.broadcast_global(cmd);
        Ok(())
    }

    pub async fn subscribe_sushi(&self) -> Result<BoxStream<MessageBroadcastInner>> {
        let stream = self.messaging.subscribe().await?;
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
}

impl ServerState {
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub fn data(&self) -> AnyData {
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

pub use crate::server::globals::{Globals, GlobalsOwned};

impl FromRef<Arc<ServerState>> for Globals {
    fn from_ref(input: &Arc<ServerState>) -> Self {
        input.globals.clone()
    }
}
