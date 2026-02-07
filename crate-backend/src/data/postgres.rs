use sqlx::PgPool;

#[derive(Debug)]
pub struct Postgres {
    pub(crate) pool: PgPool,
    // TODO: make postgres use one transaction + smaller queries?
    // pub(crate) conn: PgConnection,
}

impl Data for Postgres {}

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
mod push;
mod reaction;
mod role;
mod role_member;
mod room;
mod room_analytics;
mod room_member;
mod search;
mod session;
mod tag;
mod thread;
mod thread_member;
mod unread;
mod user;
mod user_config;
mod user_email;
mod user_relationship;
mod util;
mod webhook;

pub use util::Pagination;

// TEMP: for media migration
pub use media::{DbMedia, DbMediaData};

use crate::data::Data;
