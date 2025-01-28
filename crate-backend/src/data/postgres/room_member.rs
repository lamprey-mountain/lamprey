use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;
use types::{
    PaginationDirection, PaginationQuery, PaginationResponse, RoomMember, RoomMemberPatch,
};

use crate::error::Result;
use crate::types::{DbRoomMember, DbRoomMembership, RoomId, RoomMemberPut, UserId};

use crate::data::DataRoomMember;

use super::{Pagination, Postgres};

#[async_trait]
impl DataRoomMember for Postgres {
    // FIXME: apply other attributes in RoomMemberPut
    async fn room_member_put(&self, put: RoomMemberPut) -> Result<()> {
        let membership: DbRoomMembership = put.membership.into();
        query!(
            "INSERT INTO room_member (user_id, room_id, membership) VALUES ($1, $2, $3)",
            put.user_id.into_inner(),
            put.room_id.into_inner(),
            membership as _
        )
        .execute(&self.pool)
        .await?;
        info!("inserted room member");
        Ok(())
    }

    async fn room_member_delete(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        query!(
            "DELETE FROM room_member WHERE room_id = $1 AND user_id = $2",
            room_id.into_inner(),
            user_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        info!("deleted room member");
        Ok(())
    }

    async fn room_member_list(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomMember>> {
        let p: Pagination<_> = pagination.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let items = query_as!(
            DbRoomMember,
            r#"
        	SELECT room_id, user_id, membership as "membership: _", override_name, override_description
            FROM room_member
        	WHERE room_id = $1 AND user_id > $2 AND user_id < $3
        	ORDER BY (CASE WHEN $4 = 'f' THEN user_id END), user_id DESC LIMIT $5
        "#,
            room_id.into_inner(),
            p.after.into_inner(),
            p.before.into_inner(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            "SELECT count(*) FROM room_member WHERE room_member.room_id = $1",
            room_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = items.len() > p.limit as usize;
        let mut items: Vec<_> = items
            .into_iter()
            .take(p.limit as usize)
            .map(Into::into)
            .collect();
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
        })
    }

    async fn room_member_get(&self, room_id: RoomId, user_id: UserId) -> Result<RoomMember> {
        let item = query_as!(
            DbRoomMember,
            r#"
        	SELECT room_id, user_id, membership as "membership: _", override_name, override_description
            FROM room_member
            WHERE room_id = $1 AND user_id = $2
        "#,
            room_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(item.into())
    }

    async fn room_member_patch(
        &self,
        room_id: RoomId,
        user_id: UserId,
        patch: RoomMemberPatch,
    ) -> Result<()> {
        query!(
            r#"
            UPDATE room_member
        	SET override_name = $3, override_description = $4
            WHERE room_id = $1 AND user_id = $2
        "#,
            room_id.into_inner(),
            user_id.into_inner(),
            patch.override_name,
            patch.override_description,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(())
    }
}
