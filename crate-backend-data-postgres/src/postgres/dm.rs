use async_trait::async_trait;
use common::v1::types::{
    Channel, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse,
};
use sqlx::{query, query_file_as, query_scalar, Acquire};

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{ChannelId, DbChannel, DbChannelType, UserId};

use crate::data::DataDm;

use super::{util::Pagination, Postgres};

fn ensure_canonical(a: UserId, b: UserId) -> (UserId, UserId) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

#[async_trait]
impl DataDm for Postgres {
    async fn dm_put(
        &self,
        user_a_id: UserId,
        user_b_id: UserId,
        channel_id: ChannelId,
    ) -> Result<()> {
        let (user_a_id, user_b_id) = ensure_canonical(user_a_id, user_b_id);
        query!(
            r#"
            INSERT INTO dm (user_a_id, user_b_id, channel_id)
            VALUES ($1, $2, $3)
            ON CONFLICT ON CONSTRAINT dm_pkey DO NOTHING
            "#,
            *user_a_id,
            *user_b_id,
            *channel_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn dm_get(&self, user_a_id: UserId, user_b_id: UserId) -> Result<Option<ChannelId>> {
        let (user_a_id, user_b_id) = ensure_canonical(user_a_id, user_b_id);
        let row = query!(
            r#"
                SELECT channel_id FROM dm
                WHERE user_a_id = $1 AND user_b_id = $2
         "#,
            *user_a_id,
            *user_b_id,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.channel_id.into()))
    }

    async fn dm_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/dm_paginate.sql",
                user_id.into_inner(),
                *p.after,
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"
                SELECT count(*)
                FROM dm
                WHERE user_a_id = $1 OR user_b_id = $1
                "#,
                user_id.into_inner()
            ),
            |i: &Channel| i.last_version_id.unwrap().to_string()
        )
    }
}
