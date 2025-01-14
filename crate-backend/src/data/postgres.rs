use sqlx::PgPool;

use super::Data;

#[derive(Debug)]
pub struct Postgres {
    pub(crate) pool: PgPool,
}

impl Data for Postgres {}

mod util;
mod room;
mod room_member;
mod role;
mod role_member;
mod thread;
mod message;
mod unread;
mod permission;
mod media;
mod invite;
mod user;
mod session;

pub use util::Pagination;
