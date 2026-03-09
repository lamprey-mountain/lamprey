use async_trait::async_trait;
use sqlx::PgPool;

#[derive(Debug)]
pub struct Postgres {
    pub(crate) pool: PgPool,
}

impl Postgres {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

use crate::data::Data;
use crate::error::Result;

#[async_trait]
impl Data for Postgres {
    async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    async fn check_database(&self) -> Result<bool> {
        Ok(sqlx::query_scalar::<_, bool>("SELECT true")
            .fetch_one(&self.pool)
            .await?)
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
mod search;
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
