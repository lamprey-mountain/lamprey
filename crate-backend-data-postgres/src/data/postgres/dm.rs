use async_trait::async_trait;
use common::v1::types::{
    Channel, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse,
};
use sqlx::{query, query_file_as, query_scalar};

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{ChannelId, DbChannel, DbChannelType, UserId};

use crate::data::DataDm;

use super::{Postgres, util::Pagination};

fn ensure_canonical(a: UserId, b: UserId) -> (UserId, UserId) {
    if a < b { (a, b) } else { (b, a) }
}

#[async_trait]
impl DataDm for Postgres {
    async fn dm_put(
        &mut self,
        user_a_id: UserId,
        user_b_id: UserId,
        channel_id: ChannelId,
        creator_id: UserId,
    ) -> Result<()> {
        let mut tx = self.begin_tx().await?;
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
        .execute(tx.ext())
        .await?;

        // dms are only visible to the user that created (or opened) them
        query!(
            r#"
            INSERT INTO dm_visible (user_id, channel_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
            *creator_id,
            *channel_id,
        )
        .execute(tx.ext())
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn dm_set_hidden(
        &mut self,
        user_id: UserId,
        channel_id: ChannelId,
        hidden: bool,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        if hidden {
            query!(
                "DELETE FROM dm_visible WHERE user_id = $1 AND channel_id = $2",
                *user_id,
                *channel_id,
            )
            .execute(conn.ext())
            .await?;
        } else {
            query!(
                r#"
                INSERT INTO dm_visible (user_id, channel_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
                *user_id,
                *channel_id,
            )
            .execute(conn.ext())
            .await?;
        }
        Ok(())
    }

    async fn dm_unhide(&mut self, channel_id: ChannelId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"
            INSERT INTO dm_visible (user_id, channel_id)
            SELECT user_a_id, channel_id FROM dm WHERE channel_id = $1
            UNION ALL
            SELECT user_b_id, channel_id FROM dm WHERE channel_id = $1
            ON CONFLICT DO NOTHING
            "#,
            *channel_id,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn dm_get(&mut self, user_a_id: UserId, user_b_id: UserId) -> Result<Option<ChannelId>> {
        let mut conn = self.acquire().await?;
        let (user_a_id, user_b_id) = ensure_canonical(user_a_id, user_b_id);
        let row = query!(
            r#"
                SELECT channel_id FROM dm
                WHERE user_a_id = $1 AND user_b_id = $2
         "#,
            *user_a_id,
            *user_b_id,
        )
        .fetch_optional(conn.ext())
        .await?;
        Ok(row.map(|r| r.channel_id.into()))
    }

    async fn dm_list(
        &mut self,
        user_id: UserId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
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
