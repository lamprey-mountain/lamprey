use sqlx::PgPool;

use super::Data;

#[derive(Debug)]
pub struct Postgres {
    pub(crate) pool: PgPool,
}

impl Data for Postgres {}

mod auth;
mod invite;
mod media;
mod message;
mod permission;
mod role;
mod role_member;
mod room;
mod room_member;
mod search;
mod session;
mod thread;
mod unread;
mod user;
mod util;

pub use util::Pagination;
