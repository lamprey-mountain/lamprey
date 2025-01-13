use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId, MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch, RoomVerId, Session, SessionId, Thread, ThreadCreate, ThreadId, UserId
};

use crate::data::{
    DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember, DataSession, DataThread, DataUnread, DataUser
};

use super::{Pagination, Postgres};

#[async_trait]
impl DataSession for Postgres {
    async fn session_create(&self, create: ThreadCreate) -> Result<SessionId> { todo!() }
    async fn session_get(&self, id: SessionId) -> Result<Session> { todo!() }
    async fn session_get_by_token(&self, token: &str) -> Result<Session> { todo!() }
    async fn session_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<SessionId>,
    ) -> Result<PaginationResponse<Session>> { todo!() }
    async fn session_delete(&self, id: SessionId) -> Result<()> { todo!() }
}
