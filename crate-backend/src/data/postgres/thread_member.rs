use async_trait::async_trait;
use common::v1::types::{
    ChannelId, PaginationDirection, PaginationQuery, PaginationResponse, ThreadMember,
    ThreadMemberPut, ThreadMembership, UserId,
};
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;
use uuid::Uuid;

use crate::data::postgres::Pagination;
use crate::data::DataThreadMember;
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
            membership: match row.membership {
                DbMembership::Join => ThreadMembership::Join,
                DbMembership::Leave => ThreadMembership::Leave,
                DbMembership::Ban => ThreadMembership::Leave,
            },
            joined_at: row.joined_at.assume_utc().into(),
        }
    }
}

#[async_trait]
impl DataThreadMember for Postgres {
    async fn thread_member_put(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        _put: ThreadMemberPut,
    ) -> Result<()> {
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
        .execute(&self.pool)
        .await?;
        info!("inserted thread member");
        Ok(())
    }

    async fn thread_member_set_membership(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        membership: ThreadMembership,
    ) -> Result<()> {
        let membership: DbMembership = membership.into();
        query!(
            r#"
            UPDATE thread_member
        	SET membership = $3
            WHERE channel_id = $1 AND user_id = $2
            "#,
            *channel_id,
            *user_id,
            membership as _,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn thread_member_delete(&self, channel_id: ChannelId, user_id: UserId) -> Result<()> {
        query!(
            "DELETE FROM thread_member WHERE channel_id = $1 AND user_id = $2",
            *channel_id,
            *user_id,
        )
        .execute(&self.pool)
        .await?;
        info!("deleted thread member");
        Ok(())
    }

    async fn thread_member_get(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<ThreadMember> {
        let item = query_as!(
            DbThreadMember,
            r#"
        	SELECT
            	channel_id,
            	user_id,
            	membership as "membership: _",
            	joined_at
            FROM thread_member
            WHERE channel_id = $1 AND user_id = $2
        "#,
            *channel_id,
            *user_id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(item.into())
    }

    async fn thread_member_list(
        &self,
        channel_id: ChannelId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ThreadMember>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
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

    async fn thread_member_list_all(&self, channel_id: ChannelId) -> Result<Vec<ThreadMember>> {
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
        .fetch_all(&self.pool)
        .await?;
        Ok(items.into_iter().map(Into::into).collect())
    }
}
