use async_trait::async_trait;
use common::v1::types::emoji::Emoji;
use common::v1::types::reaction::{ReactionKey, ReactionListItem};
use common::v1::types::{
    MessageId, PaginationDirection, PaginationQuery, PaginationResponse, ThreadId,
};
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::debug;

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
        _thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    ) -> Result<()> {
        debug!("reaction put user_id={user_id} message_id={message_id} key={key:?}");
        let mut tx = self.pool.begin().await?;

        let emoji_id = match &key.0 {
            Emoji::Custom(e) => Some(*e.id),
            Emoji::Unicode(_) => None,
        };
        let key_str = match &key.0 {
            Emoji::Custom(e) => e.id.to_string(),
            Emoji::Unicode(e) => e.0.to_owned(),
        };

        // Check if the key already exists for this message by any user
        let key_exists: bool = query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM reaction WHERE message_id = $1 AND key = $2)",
            *message_id,
            &key_str
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(false);

        if !key_exists {
            // It's a new unique reaction, check the limit
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
                    (SELECT position FROM reaction WHERE message_id = $1 AND key = $3),
                    (SELECT coalesce(max(position) + 1, 0) FROM reaction WHERE message_id = $1)
                ) AS pos
            )
            INSERT INTO reaction (message_id, user_id, key, emoji_id, position)
            SELECT $1, $2, $3, $4, pos FROM pos
            ON CONFLICT DO NOTHING
            "#,
            *message_id,
            *user_id,
            &key_str,
            emoji_id,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn reaction_delete(
        &self,
        user_id: UserId,
        _thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    ) -> Result<()> {
        debug!("reaction delete user_id={user_id} message_id={message_id} key={key:?}");
        let key = match &key.0 {
            Emoji::Custom(e) => e.id.to_string(),
            Emoji::Unicode(e) => e.0.to_owned(),
        };
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
        _thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ReactionListItem>> {
        let p: Pagination<_> = pagination.try_into()?;
        let key = match &key.0 {
            Emoji::Custom(e) => e.id.to_string(),
            Emoji::Unicode(e) => e.0.to_owned(),
        };

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

    async fn reaction_purge(&self, _thread_id: ThreadId, message_id: MessageId) -> Result<()> {
        query!(r#"DELETE FROM reaction WHERE message_id = $1"#, *message_id,)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn reaction_purge_key(
        &self,
        _thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    ) -> Result<()> {
        let key = match &key.0 {
            Emoji::Custom(e) => e.id.to_string(),
            Emoji::Unicode(e) => e.0.to_owned(),
        };
        query!(
            r#"DELETE FROM reaction WHERE message_id = $1 AND key = $2"#,
            *message_id,
            key,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
