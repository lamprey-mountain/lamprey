use async_trait::async_trait;
use sqlx::PgPool;
use tracing::warn;

use crate::{
    Data,
    data::{AnyData, Database},
    error::Result,
};

/// entry point for accessing postgres
#[derive(Debug, Clone)]
pub struct PostgresPool {
    // TEMP: pub fields
    pub pool: PgPool,
}

/// a single unit of work
#[derive(Debug)]
pub struct Postgres {
    // TEMP: pub fields
    pub pool: PgPool,
    pub txn: Option<sqlx::PgTransaction<'static>>,

    /// whether to use legacy behavior
    ///
    /// - old code made the DataFoo impls begin and commit transactions themselves.
    /// - newer code pushes transaction handling up to the caller
    pub use_legacy_behavior: bool,
}

/// a way to access the postgresql database
pub(crate) enum DbHandle<'a> {
    /// Using this unit of work's transaction
    GlobalTx(&'a mut sqlx::PgConnection),

    /// acquired a temporary connection for this database query
    // TEMP: legacy behavior
    LocalConn(sqlx::pool::PoolConnection<sqlx::Postgres>),

    /// started a temporary transaction for this database query
    // TEMP: legacy behavior
    LocalTx(sqlx::Transaction<'static, sqlx::Postgres>),
}

impl PostgresPool {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Database for PostgresPool {
    async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    async fn check_database(&self) -> Result<bool> {
        Ok(sqlx::query_scalar::<_, bool>("SELECT true")
            .fetch_one(&self.pool)
            .await?)
    }

    async fn begin(&self) -> Result<AnyData> {
        let txn = self.pool.begin().await?;
        Ok(Box::new(Postgres {
            pool: self.pool.clone(),
            txn: Some(txn),
            use_legacy_behavior: false,
        }))
    }

    async fn begin_read(&self) -> Result<AnyData> {
        Ok(Box::new(Postgres {
            pool: self.pool.clone(),
            txn: None,
            use_legacy_behavior: false,
        }))
    }
}

impl Postgres {
    /// acquire a connection. use this for reads.
    pub(crate) async fn acquire(&mut self) -> Result<DbHandle<'_>> {
        if let Some(ref mut txn) = self.txn {
            Ok(DbHandle::GlobalTx(&mut **txn))
        } else {
            let conn = self.pool.acquire().await?;
            Ok(DbHandle::LocalConn(conn))
        }
    }

    /// begin a transaction. use this for writes.
    pub(crate) async fn begin_tx(&mut self) -> Result<DbHandle<'_>> {
        if let Some(ref mut txn) = self.txn {
            Ok(DbHandle::GlobalTx(&mut **txn))
        } else {
            let tx = self.pool.begin().await?;
            Ok(DbHandle::LocalTx(tx))
        }
    }
}

#[async_trait]
impl Data for Postgres {
    async fn rollback(mut self: Box<Self>) -> Result<()> {
        if let Some(txn) = self.txn.take() {
            txn.rollback().await?;
        }
        Ok(())
    }

    async fn commit(mut self: Box<Self>) -> Result<()> {
        if let Some(txn) = self.txn.take() {
            txn.commit().await?;
        }
        Ok(())
    }
}

impl Drop for Postgres {
    fn drop(&mut self) {
        if !self.use_legacy_behavior && self.txn.is_some() {
            warn!("postgres transaction implicitly rolled back");
        }
    }
}

impl<'a> DbHandle<'a> {
    /// get inner connection
    pub fn ext(&mut self) -> &mut sqlx::PgConnection {
        match self {
            DbHandle::GlobalTx(tx) => &mut **tx,
            DbHandle::LocalConn(conn) => &mut **conn,
            DbHandle::LocalTx(tx) => &mut **tx,
        }
    }

    /// commit the local transaction
    ///
    /// ignored for global transactions
    pub async fn commit(self) -> Result<()> {
        match self {
            DbHandle::GlobalTx(_) => Ok(()),  // Managed by the caller
            DbHandle::LocalConn(_) => Ok(()), // Auto-commits
            DbHandle::LocalTx(tx) => {
                tx.commit().await?;
                Ok(())
            }
        }
    }

    /// rollback the local transaction
    ///
    /// ignored for global transactions
    pub async fn rollback(self) -> Result<()> {
        match self {
            DbHandle::GlobalTx(_) => Ok(()), // Managed by the caller
            DbHandle::LocalConn(_) => Ok(()),
            DbHandle::LocalTx(tx) => {
                tx.rollback().await?;
                Ok(())
            }
        }
    }
}

mod admin;
mod application;
mod audit_logs;
mod auth;
mod automod;
mod calendar;
mod channel;
mod config_internal;
mod connection;
mod dm;
mod document;
mod email_queue;
mod embed;
mod emoji;
mod harvest;
mod invite;
mod media;
mod message;
mod metrics;
mod notification;
mod permission;
mod permission_overwrite;
mod preferences;
mod push;
mod reaction;
mod role;
mod role_member;
mod room;
mod room_analytics;
mod room_member;
mod room_template;
mod script;
mod search_queue;
mod session;
mod tag;
mod thread;
mod thread_member;
mod unread;
mod user;
mod user_email;
mod user_relationship;
pub mod util;
mod webhook;

pub use util::Pagination;

// TEMP: for media migration
pub use media::{DbMedia, DbMediaData, DbMediaWithId};

use std::ops::{Deref, DerefMut};

impl<'a> Deref for DbHandle<'a> {
    type Target = sqlx::PgConnection;

    fn deref(&self) -> &Self::Target {
        match self {
            DbHandle::GlobalTx(tx) => &**tx,
            DbHandle::LocalConn(conn) => &**conn,
            DbHandle::LocalTx(tx) => &**tx,
        }
    }
}

impl<'a> DerefMut for DbHandle<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ext()
    }
}
