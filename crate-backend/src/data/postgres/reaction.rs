use async_trait::async_trait;
use common::v1::types::emoji::Emoji;
use common::v1::types::reaction::{ReactionKey, ReactionListItem};
use common::v1::types::{
    MessageId, PaginationDirection, PaginationQuery, PaginationResponse, ThreadId,
};
use sqlx::{query, query_as, query_scalar, Acquire};

use crate::data::postgres::Pagination;
use crate::error::Result;
use crate::gen_paginate;
use crate::types::UserId;

use crate::data::DataReaction;

use super::Postgres;

#[async_trait]
impl DataReaction for Postgres {
    async fn reaction_message_put(
        &self,
        user_id: UserId,
        _thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    ) -> Result<()> {
        match &key.0 {
            Emoji::Custom(emoji_custom) => {
                query!(
                    r#"
                    INSERT INTO reaction_message_custom (message_id, user_id, reaction_key)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    "#,
                    message_id.into_inner(),
                    user_id.into_inner(),
                    emoji_custom.id.into_inner()
                )
                .execute(&self.pool)
                .await?
            }
            Emoji::Unicode(emoji_unicode) => {
                query!(
                    r#"
                    INSERT INTO reaction_message_unicode (message_id, user_id, reaction_key)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    "#,
                    message_id.into_inner(),
                    user_id.into_inner(),
                    emoji_unicode.0
                )
                .execute(&self.pool)
                .await?
            }
        };
        Ok(())
    }

    async fn reaction_message_delete(
        &self,
        user_id: UserId,
        _thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
    ) -> Result<()> {
        match &key.0 {
            Emoji::Custom(emoji_custom) => {
                query!(
                    r#"
                    DELETE FROM reaction_message_custom
                    WHERE message_id = $1 AND user_id = $2 AND reaction_key = $3
                    "#,
                    message_id.into_inner(),
                    user_id.into_inner(),
                    emoji_custom.id.into_inner()
                )
                .execute(&self.pool)
                .await?
            }
            Emoji::Unicode(emoji_unicode) => {
                query!(
                    r#"
                    DELETE FROM reaction_message_unicode
                    WHERE message_id = $1 AND user_id = $2 AND reaction_key = $3
                    "#,
                    message_id.into_inner(),
                    user_id.into_inner(),
                    emoji_unicode.0
                )
                .execute(&self.pool)
                .await?
            }
        };
        Ok(())
    }

    async fn reaction_message_list(
        &self,
        _thread_id: ThreadId,
        message_id: MessageId,
        key: ReactionKey,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ReactionListItem>> {
        let p: Pagination<_> = pagination.try_into()?;
        match &key.0 {
            Emoji::Custom(emoji_custom) => {
                gen_paginate!(
                    p,
                    self.pool,
                    query_as!(
                        ReactionListItem,
                        r#"
                        SELECT user_id FROM reaction_message_custom
                        WHERE message_id = $1 AND reaction_key = $2 AND user_id > $3 AND user_id < $4
                    	ORDER BY (CASE WHEN $5 = 'f' THEN user_id END), user_id DESC LIMIT $6
                        "#,
                        message_id.into_inner(),
                        emoji_custom.id.into_inner(),
                        p.after.into_inner(),
                        p.before.into_inner(),
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!(
                        r#"SELECT count(*) FROM reaction_message_custom WHERE message_id = $1 AND reaction_key = $2"#,
                        message_id.into_inner(),
                        emoji_custom.id.into_inner(),
                    )
                )
            }
            Emoji::Unicode(emoji_unicode) => {
                gen_paginate!(
                    p,
                    self.pool,
                    query_as!(
                        ReactionListItem,
                        r#"
                        SELECT user_id FROM reaction_message_unicode
                        WHERE message_id = $1 AND reaction_key = $2 AND user_id > $3 AND user_id < $4
                    	ORDER BY (CASE WHEN $5 = 'f' THEN user_id END), user_id DESC LIMIT $6
                        "#,
                        message_id.into_inner(),
                        emoji_unicode.0,
                        p.after.into_inner(),
                        p.before.into_inner(),
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!(
                        r#"SELECT count(*) FROM reaction_message_unicode WHERE message_id = $1 AND reaction_key = $2"#,
                        message_id.into_inner(),
                        emoji_unicode.0,
                    )
                )
            }
        }
    }

    async fn reaction_message_purge(
        &self,
        _thread_id: ThreadId,
        message_id: MessageId,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        query!(
            r#"DELETE FROM reaction_message_custom WHERE message_id = $1"#,
            message_id.into_inner(),
        )
        .execute(&mut *tx)
        .await?;
        query!(
            r#"DELETE FROM reaction_message_unicode WHERE message_id = $1"#,
            message_id.into_inner(),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn reaction_thread_put(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        key: ReactionKey,
    ) -> Result<()> {
        match &key.0 {
            Emoji::Custom(emoji_custom) => {
                query!(
                    r#"
                    INSERT INTO reaction_thread_custom (thread_id, user_id, reaction_key)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    "#,
                    thread_id.into_inner(),
                    user_id.into_inner(),
                    emoji_custom.id.into_inner()
                )
                .execute(&self.pool)
                .await?
            }
            Emoji::Unicode(emoji_unicode) => {
                query!(
                    r#"
                    INSERT INTO reaction_thread_unicode (thread_id, user_id, reaction_key)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    "#,
                    thread_id.into_inner(),
                    user_id.into_inner(),
                    emoji_unicode.0
                )
                .execute(&self.pool)
                .await?
            }
        };
        Ok(())
    }

    async fn reaction_thread_delete(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        key: ReactionKey,
    ) -> Result<()> {
        match &key.0 {
            Emoji::Custom(emoji_custom) => {
                query!(
                    r#"
                    DELETE FROM reaction_thread_custom
                    WHERE thread_id = $1 AND user_id = $2 AND reaction_key = $3
                    "#,
                    thread_id.into_inner(),
                    user_id.into_inner(),
                    emoji_custom.id.into_inner()
                )
                .execute(&self.pool)
                .await?
            }
            Emoji::Unicode(emoji_unicode) => {
                query!(
                    r#"
                    DELETE FROM reaction_thread_unicode
                    WHERE thread_id = $1 AND user_id = $2 AND reaction_key = $3
                    "#,
                    thread_id.into_inner(),
                    user_id.into_inner(),
                    emoji_unicode.0
                )
                .execute(&self.pool)
                .await?
            }
        };
        Ok(())
    }

    async fn reaction_thread_list(
        &self,
        thread_id: ThreadId,
        key: ReactionKey,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<ReactionListItem>> {
        let p: Pagination<_> = pagination.try_into()?;
        match &key.0 {
            Emoji::Custom(emoji_custom) => {
                gen_paginate!(
                    p,
                    self.pool,
                    query_as!(
                        ReactionListItem,
                        r#"
                        SELECT user_id FROM reaction_thread_custom
                        WHERE thread_id = $1 AND reaction_key = $2 AND user_id > $3 AND user_id < $4
                    	ORDER BY (CASE WHEN $5 = 'f' THEN user_id END), user_id DESC LIMIT $6
                        "#,
                        thread_id.into_inner(),
                        emoji_custom.id.into_inner(),
                        p.after.into_inner(),
                        p.before.into_inner(),
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!(
                        r#"SELECT count(*) FROM reaction_thread_custom WHERE thread_id = $1 AND reaction_key = $2"#,
                        thread_id.into_inner(),
                        emoji_custom.id.into_inner(),
                    )
                )
            }
            Emoji::Unicode(emoji_unicode) => {
                gen_paginate!(
                    p,
                    self.pool,
                    query_as!(
                        ReactionListItem,
                        r#"
                        SELECT user_id FROM reaction_thread_unicode
                        WHERE thread_id = $1 AND reaction_key = $2 AND user_id > $3 AND user_id < $4
                    	ORDER BY (CASE WHEN $5 = 'f' THEN user_id END), user_id DESC LIMIT $6
                        "#,
                        thread_id.into_inner(),
                        emoji_unicode.0,
                        p.after.into_inner(),
                        p.before.into_inner(),
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!(
                        r#"SELECT count(*) FROM reaction_thread_unicode WHERE thread_id = $1 AND reaction_key = $2"#,
                        thread_id.into_inner(),
                        emoji_unicode.0,
                    )
                )
            }
        }
    }

    async fn reaction_thread_purge(&self, thread_id: ThreadId) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        query!(
            r#"DELETE FROM reaction_thread_custom WHERE thread_id = $1"#,
            thread_id.into_inner(),
        )
        .execute(&mut *tx)
        .await?;
        query!(
            r#"DELETE FROM reaction_thread_unicode WHERE thread_id = $1"#,
            thread_id.into_inner(),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
