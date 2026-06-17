use async_trait::async_trait;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{
    ChannelId, PaginationDirection, PaginationQuery, PaginationResponse, ThreadMember,
    ThreadMemberPut, UserId,
};
use lamprey_backend_core::Error;
use sqlx::{query, query_as, query_scalar};
use tracing::info;
use uuid::Uuid;

use crate::data::DataThreadMember;
use crate::data::postgres::Pagination;
use crate::error::Result;
use crate::gen_paginate;
use crate::types::DbMembership;

use super::Postgres;

pub struct DbThreadMember {
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub membership: DbMembership,
    pub joined_at: time::PrimitiveDateTime,
}

impl From<DbThreadMember> for ThreadMember {
    fn from(row: DbThreadMember) -> Self {
        Self {
            user_id: row.user_id.into(),
            thread_id: row.channel_id.into(),
            joined_at: row.joined_at.assume_utc().into(),
        }
    }
}

#[async_trait]
impl DataThreadMember for Postgres {
    async fn thread_member_put(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
        _put: ThreadMemberPut,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"
            INSERT INTO thread_member (user_id, channel_id, membership, joined_at)
            VALUES ($1, $2, $3, now())
			ON CONFLICT ON CONSTRAINT thread_member_pkey DO UPDATE SET
    			membership = excluded.membership,
                joined_at = case
                    when excluded.membership = 'Leave'
                    then now()
                    else thread_member.joined_at
                end
            "#,
            *user_id,
            *channel_id,
            DbMembership::Join as _,
        )
        .execute(conn.ext())
        .await?;
        info!("inserted thread member");
        Ok(())
    }

    async fn thread_member_leave(&mut self, channel_id: ChannelId, user_id: UserId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"
            UPDATE thread_member
        	SET membership = 'Leave'
            WHERE channel_id = $1 AND user_id = $2
            "#,
            *channel_id,
            *user_id,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn thread_member_delete(&mut self, channel_id: ChannelId, user_id: UserId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "DELETE FROM thread_member WHERE channel_id = $1 AND user_id = $2",
            *channel_id,
            *user_id,
        )
        .execute(conn.ext())
        .await?;
        info!("deleted thread member");
        Ok(())
    }

    async fn thread_member_get(
        &mut self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<ThreadMember> {
        let mut conn = self.acquire().await?;
        let item = query_as!(
            DbThreadMember,
            r#"
        	SELECT
            	channel_id,
            	user_id,
            	membership as "membership: _",
            	joined_at
            FROM thread_member
            WHERE channel_id = $1 AND user_id = $2 AND membership = 'Join'
        "#,
            *channel_id,
            *user_id,
        )
        .fetch_one(conn.ext())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                Error::ApiError(ApiError::from_code(ErrorCode::UnknownThreadMember))
            }
            e => Error::Sqlx(e),
        })?;
        Ok(item.into())
    }

    async fn thread_member_get_many(
        &mut self,
        thread_id: ChannelId,
        user_ids: &[UserId],
    ) -> Result<Vec<ThreadMember>> {
        let mut conn = self.acquire().await?;
        let user_ids: Vec<Uuid> = user_ids.iter().map(|id| id.into_inner()).collect();
        let items = query_as!(
            DbThreadMember,
            r#"
        	SELECT
            	channel_id,
            	user_id,
            	membership as "membership: _",
            	joined_at
            FROM thread_member
            WHERE channel_id = $1 AND user_id = ANY($2::uuid[]) AND membership = 'Join'
        "#,
            *thread_id,
            &user_ids
        )
        .fetch_all(conn.ext())
        .await?;
        Ok(items.into_iter().map(Into::into).collect())
    }

    async fn thread_member_list(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ThreadMember>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
            query_as!(
                DbThreadMember,
                r#"
            	SELECT
                	channel_id,
                	user_id,
                	membership as "membership: _",
                	joined_at
                FROM thread_member
            	WHERE channel_id = $1 AND user_id > $2 AND user_id < $3 AND membership = 'Join'
            	ORDER BY (CASE WHEN $4 = 'f' THEN user_id END), user_id DESC LIMIT $5
                "#,
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM thread_member WHERE channel_id = $1 AND membership = 'Join'",
                *channel_id
            ),
            |i: &ThreadMember| i.user_id.to_string()
        )
    }

    async fn thread_member_list_all(&mut self, channel_id: ChannelId) -> Result<Vec<ThreadMember>> {
        let mut conn = self.acquire().await?;
        let items = query_as!(
            DbThreadMember,
            r#"
            SELECT
                channel_id,
                user_id,
                membership as "membership: _",
                joined_at
            FROM thread_member
            WHERE channel_id = $1 AND membership = 'Join'
            "#,
            *channel_id,
        )
        .fetch_all(conn.ext())
        .await?;
        Ok(items.into_iter().map(Into::into).collect())
    }

    async fn thread_member_bulk_fetch(
        &mut self,
        user_id: UserId,
        thread_ids: &[ChannelId],
    ) -> Result<Vec<(ChannelId, ThreadMember)>> {
        let mut conn = self.acquire().await?;
        let thread_uuids: Vec<Uuid> = thread_ids.iter().map(|id| id.into_inner()).collect();
        if thread_uuids.is_empty() {
            return Ok(vec![]);
        }
        let items = query_as!(
            DbThreadMember,
            r#"
            SELECT
                channel_id,
                user_id,
                membership as "membership: _",
                joined_at
            FROM thread_member
            WHERE user_id = $1 AND channel_id = ANY($2) AND membership = 'Join'
            "#,
            user_id.into_inner(),
            &thread_uuids
        )
        .fetch_all(conn.ext())
        .await?;

        let result = items
            .into_iter()
            .map(|db_member| {
                let thread_member: ThreadMember = db_member.into();
                (thread_member.thread_id, thread_member)
            })
            .collect();

        Ok(result)
    }
}
