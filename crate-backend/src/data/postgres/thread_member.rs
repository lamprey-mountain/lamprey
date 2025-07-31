use async_trait::async_trait;
use common::v1::types::{
    PaginationDirection, PaginationQuery, PaginationResponse, ThreadId, ThreadMember,
    ThreadMemberPatch, ThreadMembership, UserId,
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
    pub thread_id: Uuid,
    pub membership: DbMembership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    // override_avatar: z.string().url().or(z.literal("")),
    pub membership_updated_at: time::PrimitiveDateTime,
}

impl From<DbThreadMember> for ThreadMember {
    fn from(row: DbThreadMember) -> Self {
        Self {
            user_id: row.user_id.into(),
            thread_id: row.thread_id.into(),
            membership: match row.membership {
                DbMembership::Join => ThreadMembership::Join {
                    override_name: row.override_name,
                    override_description: row.override_description,
                },
                DbMembership::Leave => ThreadMembership::Leave {},
                DbMembership::Ban => ThreadMembership::Ban {},
            },
            membership_updated_at: row.membership_updated_at.assume_utc().into(),
        }
    }
}

#[async_trait]
impl DataThreadMember for Postgres {
    async fn thread_member_put(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        membership: ThreadMembership,
    ) -> Result<()> {
        let membership: DbMembership = membership.into();
        query!(
            r#"
            INSERT INTO thread_member (user_id, thread_id, membership)
            VALUES ($1, $2, $3)
			ON CONFLICT ON CONSTRAINT thread_member_pkey DO UPDATE SET
    			membership = excluded.membership
            "#,
            user_id.into_inner(),
            thread_id.into_inner(),
            membership as _
        )
        .execute(&self.pool)
        .await?;
        info!("inserted thread member");
        Ok(())
    }

    async fn thread_member_patch(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        patch: ThreadMemberPatch,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let item = query_as!(
            DbThreadMember,
            r#"
        	SELECT
            	thread_id,
            	user_id,
            	membership as "membership: _",
            	membership_updated_at,
            	override_name,
            	override_description 
            FROM thread_member
            WHERE thread_id = $1 AND user_id = $2
        "#,
            thread_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_one(&mut *tx)
        .await?;
        query!(
            r#"
            UPDATE thread_member
        	SET override_name = $3, override_description = $4
            WHERE thread_id = $1 AND user_id = $2 AND membership = 'Join'
        "#,
            thread_id.into_inner(),
            user_id.into_inner(),
            patch.override_name.unwrap_or(item.override_name),
            patch
                .override_description
                .unwrap_or(item.override_description),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn thread_member_set_membership(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        membership: ThreadMembership,
    ) -> Result<()> {
        let membership: DbMembership = membership.into();
        query!(
            r#"
            UPDATE thread_member
        	SET membership = $3, membership_updated_at = now()
            WHERE thread_id = $1 AND user_id = $2
            "#,
            thread_id.into_inner(),
            user_id.into_inner(),
            membership as _,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn thread_member_delete(&self, thread_id: ThreadId, user_id: UserId) -> Result<()> {
        query!(
            "DELETE FROM thread_member WHERE thread_id = $1 AND user_id = $2",
            thread_id.into_inner(),
            user_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        info!("deleted thread member");
        Ok(())
    }

    async fn thread_member_get(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
    ) -> Result<ThreadMember> {
        let item = query_as!(
            DbThreadMember,
            r#"
        	SELECT
            	thread_id,
            	user_id,
            	membership as "membership: _",
            	membership_updated_at,
            	override_name,
            	override_description 
            FROM thread_member
            WHERE thread_id = $1 AND user_id = $2
        "#,
            thread_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(item.into())
    }

    async fn thread_member_list(
        &self,
        thread_id: ThreadId,
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
                	thread_id,
                	user_id,
                	membership as "membership: _",
                    membership_updated_at,
                	override_name,
                    override_description
                FROM thread_member
            	WHERE thread_id = $1 AND user_id > $2 AND user_id < $3 AND membership = 'Join'
            	ORDER BY (CASE WHEN $4 = 'f' THEN user_id END), user_id DESC LIMIT $5
                "#,
                thread_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM thread_member WHERE thread_id = $1 AND membership = 'Join'",
                thread_id.into_inner()
            ),
            |i: &ThreadMember| i.user_id.to_string()
        )
    }
}
