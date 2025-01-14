use async_trait::async_trait;
use sqlx::{query, Acquire};
use tracing::info;

use crate::error::{Error, Result};
use crate::types::{
    RoomId, RoomMemberPut, UserId
};

use crate::data::
    DataRoomMember
;

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
