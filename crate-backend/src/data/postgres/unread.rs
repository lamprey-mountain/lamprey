use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId,
    MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse,
    Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch,
    RoomVerId, Thread, ThreadCreate, ThreadId, UserId,
};

use crate::data::{
    DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember,
    DataThread, DataUnread,
};

use super::{Pagination, Postgres};

#[async_trait]
impl DataUnread for Postgres {
    async fn unread_put(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        version_id: MessageVerId,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query!(r#"
			INSERT INTO unread (thread_id, user_id, version_id)
			VALUES ($1, $2, $3)
			ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET version_id = excluded.version_id;
        "#, thread_id.into_inner(), user_id.into_inner(), version_id.into_inner())
        .execute(&mut *conn)
        .await?;
        Ok(())
    }
}
