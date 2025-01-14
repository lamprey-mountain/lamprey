use async_trait::async_trait;
use sqlx::query;
use tracing::info;

use crate::error::Result;
use crate::types::{DbRoomMembership, RoomId, RoomMemberPut, UserId};

use crate::data::DataRoomMember;

use super::Postgres;

#[async_trait]
impl DataRoomMember for Postgres {
    async fn room_member_put(&self, put: RoomMemberPut) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let membership: DbRoomMembership = put.membership.into();
        query!(
            "INSERT INTO room_member (user_id, room_id, membership) VALUES ($1, $2, $3)",
            put.user_id.into_inner(),
            put.room_id.into_inner(),
            membership as _
        )
        .execute(&mut *conn)
        .await?;
        info!("inserted room member");
        Ok(())
    }

    async fn room_member_delete(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query!(
            "DELETE FROM room_member WHERE room_id = $1 AND user_id = $2",
            room_id.into_inner(),
            user_id.into_inner(),
        )
        .execute(&mut *conn)
        .await?;
        info!("deleted room member");
        Ok(())
    }
}
