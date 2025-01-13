use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId, MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch, RoomVerId, Thread, ThreadCreate, ThreadId, UserId
};

use crate::data::{
    DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember, DataThread, DataUnread
};

use super::Postgres;

#[async_trait]
impl DataRoleMember for Postgres {
    async fn role_member_put(&self, user_id: UserId, role_id: RoleId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query_as!(
            Role,
            r#"
            INSERT INTO role_member (user_id, role_id)
    		VALUES ($1, $2)
        "#,
            user_id.into_inner(),
            role_id.into_inner()
        )
        .execute(&mut *conn)
        .await?;
        info!("inserted role member");
        Ok(())
    }

    async fn role_member_delete(&self, user_id: UserId, role_id: RoleId)
        -> Result<()> {
        todo!()
    }
}
