use async_trait::async_trait;
use common::v1::types::{PaginationDirection, PaginationQuery, PaginationResponse, RoomMember};
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;

use crate::error::Result;
use crate::types::{RoleId, UserId};

use crate::data::DataRoleMember;

use super::room_member::DbRoomMember;
use super::{Pagination, Postgres};

#[async_trait]
impl DataRoleMember for Postgres {
    async fn role_member_put(&self, user_id: UserId, role_id: RoleId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query!(
            "INSERT INTO role_member (user_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            user_id.into_inner(),
            role_id.into_inner()
        )
        .execute(&mut *conn)
        .await?;
        info!("inserted role member");
        Ok(())
    }

    async fn role_member_delete(&self, user_id: UserId, role_id: RoleId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query!(
            "DELETE FROM role_member WHERE role_id = $1 AND user_id = $2",
            role_id.into_inner(),
            user_id.into_inner(),
        )
        .execute(&mut *conn)
        .await?;
        info!("deleted role member");
        Ok(())
    }

    async fn role_member_list(
        &self,
        role_id: RoleId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomMember>> {
        let p: Pagination<_> = paginate.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let items = query_as!(
            DbRoomMember,
            r#"
            with ro as (
                select user_id, array_agg(role_id) as roles from role_member
                join role on role.room_id = $1 and role_member.role_id = role.id
                group by user_id
            )
        	SELECT
            	r.user_id,
            	r.room_id,
                r.membership as "membership: _",
                r.override_name,
                r.override_description,
                r.joined_at,
                r.origin,
                r.mute,
                r.deaf,
                r.timeout_until,
            	coalesce(ro.roles, '{}') as "roles!"
            FROM role_member AS m
            JOIN role ON role.id = m.role_id
            JOIN room_member r ON r.room_id = role.room_id AND r.user_id = m.user_id
            left join ro on ro.user_id = m.user_id
        	WHERE m.role_id = $1 AND r.user_id > $2 AND r.user_id < $3
        	ORDER BY (CASE WHEN $4 = 'f' THEN r.user_id END), r.user_id DESC LIMIT $5
        "#,
            role_id.into_inner(),
            p.after.into_inner(),
            p.before.into_inner(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            "SELECT count(*) FROM role_member WHERE role_id = $1",
            role_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = items.len() > p.limit as usize;
        let mut items: Vec<RoomMember> = items
            .into_iter()
            .take(p.limit as usize)
            .map(Into::into)
            .collect();
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        let cursor = items.last().map(|i| i.user_id.to_string());
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
            cursor,
        })
    }

    async fn role_member_count(&self, role_id: RoleId) -> Result<u64> {
        let total = query_scalar!(
            "SELECT count(*) FROM role_member WHERE role_id = $1",
            role_id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(total.unwrap_or(0) as u64)
    }

    async fn role_member_bulk_edit(
        &self,
        role_id: RoleId,
        apply_user_ids: &[UserId],
        remove_user_ids: &[UserId],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if !apply_user_ids.is_empty() {
            let apply_user_ids: Vec<uuid::Uuid> =
                apply_user_ids.iter().map(|id| id.into_inner()).collect();
            query!(
                "INSERT INTO role_member (user_id, role_id) SELECT unnest($1::uuid[]), $2 ON CONFLICT DO NOTHING",
                &apply_user_ids,
                role_id.into_inner()
            )
            .execute(&mut *tx)
            .await?;
        }

        if !remove_user_ids.is_empty() {
            let remove_user_ids: Vec<uuid::Uuid> =
                remove_user_ids.iter().map(|id| id.into_inner()).collect();
            query!(
                "DELETE FROM role_member WHERE role_id = $1 AND user_id = ANY($2::uuid[])",
                role_id.into_inner(),
                &remove_user_ids,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
