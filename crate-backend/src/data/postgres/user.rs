use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId, MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch, RoomVerId, Thread, ThreadCreate, ThreadId, User, UserCreate, UserId, UserPatch
};

use crate::data::{
    DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember, DataThread, DataUnread, DataUser
};

use super::{Pagination, Postgres};

#[async_trait]
impl DataUser for Postgres {
    async fn user_insert(&self, id: UserId, patch: UserCreate) -> Result<UserId> { todo!() }
    async fn user_update(&self, id: UserId, patch: UserPatch) -> Result<User> { todo!() }
    async fn user_delete(&self, id: UserId) -> Result<()> { todo!() }
    
    async fn user_get(&self, id: UserId) -> Result<User> {
        let mut conn = self.pool.acquire().await?;
        let row = query!(r#"
            SELECT id, parent_id, name, description, status, is_bot, is_alias, is_system
            FROM usr WHERE id = $1
        "#, id.into_inner())
            .fetch_one(&mut *conn)
            .await?;
        let user = User {
            id,
            parent_id: row.parent_id.map(UserId),
            name: row.name,
            description: row.description,
            status: row.status,
            is_bot: row.is_bot,
            is_alias: row.is_alias,
            is_system: row.is_system,
        };
        Ok(user)
    }
}
