use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Invite, InviteCode, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId, MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch, RoomVerId, Thread, ThreadCreate, ThreadId, UserId
};

use crate::data::{
    DataInvite, DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember, DataThread, DataUnread
};

use super::{Pagination, Postgres};

#[async_trait]
impl DataInvite for Postgres {
	async fn invite_insert_room(&self, room_id: RoomId, creator_id: UserId, code: InviteCode) -> Result<Invite> { todo!() }
	async fn invite_select(&self, code: InviteCode) -> Result<Invite> { todo!() }
	async fn invite_delete(&self, code: InviteCode) -> Result<()> { todo!() }
}
