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
impl DataRoomMember for Postgres {
    async fn room_member_put(&self, put: RoomMemberPut) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query!(
            "
    	    INSERT INTO room_member (user_id, room_id, membership)
    	    VALUES ($1, $2, $3)
        ",
            put.user_id.into_inner(),
            put.room_id.into_inner(),
            put.membership as _
        )
        .execute(&mut *conn)
        .await?;
        info!("inserted room member");
        Ok(())
    }

    async fn room_member_delete(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        todo!()
    }
}
