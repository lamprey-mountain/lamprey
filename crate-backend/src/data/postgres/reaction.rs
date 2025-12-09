use async_trait::async_trait;
use common::v1::types::reaction::{ReactionKeyParam, ReactionListItem};
use common::v1::types::{
    ChannelId, MessageId, PaginationDirection, PaginationQuery, PaginationResponse,
};
use serde::Deserialize;
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::debug;
use uuid::Uuid;

use crate::data::postgres::Pagination;
use crate::error::Result;
use crate::gen_paginate;
use crate::types::UserId;

use crate::data::DataReaction;

use super::Postgres;

#[async_trait]
impl DataReaction for Postgres {
    async fn reaction_put(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
    ) -> Result<()> {
        debug!("reaction put user_id={user_id} message_id={message_id} key={key:?}");
        let mut tx = self.pool.begin().await?;
        let key_str = key.to_string();

        let key_exists: bool = query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM reaction WHERE message_id = $1 AND key = $2)",
            *message_id,
            &key_str
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(false);

        if !key_exists {
            // new reaction, check limit
            let unique_reaction_count: i64 = query_scalar!(
                "SELECT count(DISTINCT key) FROM reaction WHERE message_id = $1",
                *message_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(0);

            if unique_reaction_count as u32 >= crate::consts::MAX_UNIQUE_REACTIONS {
                return Err(crate::Error::BadRequest(format!(
                    "too many unique reactions (max {})",
                    crate::consts::MAX_UNIQUE_REACTIONS
                )));
            }
        }

        query!(
            r#"
            WITH pos AS (
                SELECT coalesce(
                    (SELECT position FROM reaction WHERE message_id = $1 AND key = $4),
                    (SELECT coalesce(max(position) + 1, 0) FROM reaction WHERE message_id = $1)
                ) AS pos
            )
            INSERT INTO reaction (message_id, user_id, channel_id, key, position)
            SELECT $1, $2, $3, $4, pos FROM pos
            ON CONFLICT DO NOTHING
            "#,
            *message_id,
            *user_id,
            *channel_id,
            key_str,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn reaction_delete(
        &self,
        user_id: UserId,
        _channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
    ) -> Result<()> {
        debug!("reaction delete user_id={user_id} message_id={message_id} key={key:?}");
        let key = key.to_string();
        query!(
            r#"
            DELETE FROM reaction
            WHERE message_id = $1 AND user_id = $2 AND key = $3
            "#,
            *message_id,
            *user_id,
            key,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn reaction_list(
        &self,
        _channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ReactionListItem>> {
        let p: Pagination<_> = pagination.try_into()?;
        let key = key.to_string();

        gen_paginate!(
            p,
            self.pool,
            query_as!(
                ReactionListItem,
                r#"
                SELECT user_id FROM reaction
                WHERE message_id = $1 AND key = $2 AND user_id > $3 AND user_id < $4
            	ORDER BY (CASE WHEN $5 = 'f' THEN user_id END), user_id DESC LIMIT $6
                "#,
                *message_id,
                key,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"SELECT count(*) FROM reaction WHERE message_id = $1 AND key = $2"#,
                *message_id,
                key,
            ),
            |i: &ReactionListItem| i.user_id.to_string()
        )
    }

    async fn reaction_delete_key(
        &self,
        _channel_id: ChannelId,
        message_id: MessageId,
        key: ReactionKeyParam,
    ) -> Result<()> {
        let key = key.to_string();
        query!(
            r#"DELETE FROM reaction WHERE message_id = $1 AND key = $2"#,
            *message_id,
            key,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn reaction_delete_all(
        &self,
        _channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<()> {
        query!(r#"DELETE FROM reaction WHERE message_id = $1"#, *message_id,)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // TODO: refactor to make this code less horrible
    async fn reaction_fetch_all(
        &self,
        _channel_id: ChannelId,
        user_id: UserId,
        messages: &[MessageId],
    ) -> Result<Vec<(MessageId, Vec<(ReactionKeyParam, u64, bool)>)>> {
        let message_ids: Vec<Uuid> = messages.iter().map(|id| id.into_inner()).collect();
        let reactions = query!(r#"
            with reaction_counts as (
                select message_id, key, min(position) as pos, count(*) as count, bool_or(user_id = $1) as self_reacted
                from reaction
                group by message_id, key
            )
            select message_id,
                json_agg(jsonb_build_object(
                    'key', key,
                    'count', count,
                    'self_reacted', self_reacted
                ) order by pos) as json
            from reaction_counts
            where message_id = any($2)
            group by message_id
            "#,
            *user_id,
            &message_ids,
        )
            .fetch_all(&self.pool)
            .await?;

        #[derive(Deserialize)]
        struct ReactionData {
            key: ReactionKeyParam,
            count: u64,
            self_reacted: bool,
        }

        let formatted = reactions
            .into_iter()
            .map(|r| {
                let data: Vec<ReactionData> = serde_json::from_value(r.json.unwrap()).unwrap();
                (
                    r.message_id.into(),
                    data.into_iter()
                        .map(|d| (d.key, d.count, d.self_reacted))
                        .collect(),
                )
            })
            .collect();

        Ok(formatted)
    }
}
