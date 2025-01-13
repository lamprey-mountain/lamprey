// TODO/TEMP: suppress warnings
#![allow(unused)]

use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId, MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch, RoomVerId, Thread, ThreadCreate, ThreadId, UserId
};

use super::{
    Data, DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember, DataThread, DataUnread
};

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
