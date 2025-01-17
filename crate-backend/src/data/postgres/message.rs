use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    DbMessage, DbMessageType, Message, MessageCreate, MessageId, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse, ThreadId
};

use crate::data::DataMessage;

use super::{Pagination, Postgres};

#[async_trait]
impl DataMessage for Postgres {
    async fn message_create(&self, create: MessageCreate) -> Result<MessageId> {
        let message_id = Uuid::now_v7();
        let atts: Vec<Uuid> = create
            .attachment_ids
            .iter()
            .map(|i| i.into_inner())
            .collect();
        let message_type: DbMessageType = create.message_type.into();
        query!(r#"
    	    INSERT INTO message (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id, type, override_name, attachments)
    	    VALUES ($1, $2, $3, (SELECT coalesce(max(ordering), 0) FROM message WHERE thread_id = $2), $4, $5, $6, $7, $8, $9, $10)
        "#, message_id, create.thread_id.into_inner(), message_id, create.content, create.metadata, create.reply_id.map(|i| i.into_inner()), create.author_id.into_inner(), message_type as _, create.override_name, &atts)
        .execute(&self.pool)
        .await?;
        info!("insert message");
        Ok(message_id.into())
    }

    async fn message_update(
        &self,
        _thread_id: ThreadId,
        message_id: MessageId,
        create: MessageCreate,
    ) -> Result<MessageVerId> {
        let ver_id = Uuid::now_v7();
        let atts: Vec<Uuid> = create
            .attachment_ids
            .iter()
            .map(|i| i.into_inner())
            .collect();
        let message_type: DbMessageType = create.message_type.into();
        query!(r#"
    	    INSERT INTO message (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id, type, override_name, attachments)
    	    VALUES ($1, $2, $3, (SELECT coalesce(max(ordering), 0) FROM message WHERE thread_id = $2), $4, $5, $6, $7, $8, $9, $10)
        "#,
            message_id.into_inner(),
            create.thread_id.into_inner(),
            ver_id,
            create.content,
            create.metadata,
            create.reply_id.map(|i| i.into_inner()),
            create.author_id.into_inner(),
            message_type as _,
            create.override_name,
            &atts,
        )
        .execute(&self.pool)
        .await?;
        Ok(ver_id.into())
    }

    async fn message_get(&self, thread_id: ThreadId, id: MessageId) -> Result<Message> {
        let row = query_as!(DbMessage, r#"
            with
            att_unnest as (select version_id, unnest(attachments) as media_id from message),
            att_json as (
                select version_id, json_agg(row_to_json(media)) as attachments
                from att_unnest
                join media on att_unnest.media_id = media.id
                group by att_unnest.version_id
            ),
            message_coalesced as (
                select *
                from (select *, row_number() over(partition by id order by version_id desc) as row_num
                    from message)
                where row_num = 1
            )
            SELECT
                msg.type as "message_type: DbMessageType",
                msg.id,
                msg.thread_id, 
                msg.version_id,
                msg.ordering,
                msg.content,
                msg.metadata,
                msg.reply_id,
                msg.override_name,
                false as "is_pinned!",
                row_to_json(usr) as "author!",
                coalesce(att_json.attachments, '[]'::json) as "attachments!"
            FROM message_coalesced AS msg
            JOIN usr ON usr.id = msg.author_id
            left JOIN att_json ON att_json.version_id = msg.version_id
                 WHERE thread_id = $1 AND msg.id = $2 AND msg.deleted_at IS NULL
        "#, thread_id.into_inner(), id.into_inner()).fetch_one(&self.pool).await?;
        Ok(row.into())
    }

    async fn message_list(
        &self,
        thread_id: ThreadId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let items = query_as!(
            DbMessage,
            r#"
            with
            att_unnest as (select version_id, unnest(attachments) as media_id from message),
            att_json as (
                select version_id, json_agg(row_to_json(media)) as attachments
                from att_unnest
                join media on att_unnest.media_id = media.id
                group by att_unnest.version_id
            ),
            message_coalesced as (
                select *
                from (select *, row_number() over(partition by id order by version_id desc) as row_num
                    from message)
                where row_num = 1
            )
        select
            msg.type as "message_type: DbMessageType",
            msg.id,
            msg.thread_id, 
            msg.version_id,
            msg.ordering,
            msg.content,
            msg.metadata,
            msg.reply_id,
            msg.override_name,
            row_to_json(usr) as "author!: serde_json::Value",
            coalesce(att_json.attachments, '[]'::json) as "attachments!: serde_json::Value",
            false as "is_pinned!"
        from message_coalesced as msg
        join usr on usr.id = msg.author_id
        left join att_json on att_json.version_id = msg.version_id
        where thread_id = $1 and msg.deleted_at is null
          and msg.id > $2 AND msg.id < $3
		order by (CASE WHEN $4 = 'F' THEN msg.id END), msg.id DESC LIMIT $5
            "#,
            thread_id.into_inner(),
            p.after.into_inner(),
            p.before.into_inner(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            r#"
            with message_coalesced as (
                select *
                from (select *, row_number() over(partition by id order by version_id desc) as row_num
                    from message)
                where row_num = 1
            )
            select count(*) from message_coalesced where thread_id = $1
            "#,
            thread_id.into_inner()
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

    async fn message_delete(&self, _thread_id: ThreadId, message_id: MessageId) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        query!(
            "UPDATE message SET deleted_at = $2 WHERE id = $1",
            message_id.into_inner(),
            now
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn message_version_get(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> Result<Message> {
        let row = query_as!(DbMessage, r#"
            with
            att_unnest as (select version_id, unnest(attachments) as media_id from message),
            att_json as (
                select version_id, json_agg(row_to_json(media)) as attachments
                from att_unnest
                join media on att_unnest.media_id = media.id
                group by att_unnest.version_id
            )
            SELECT
                msg.type as "message_type: DbMessageType",
                msg.id,
                msg.thread_id, 
                msg.version_id,
                msg.ordering,
                msg.content,
                msg.metadata,
                msg.reply_id,
                msg.override_name,
                false as "is_pinned!",
                row_to_json(usr) as "author!",
                coalesce(att_json.attachments, '[]'::json) as "attachments!"
            FROM message AS msg
            JOIN usr ON usr.id = msg.author_id
            left JOIN att_json ON att_json.version_id = msg.version_id
                 WHERE thread_id = $1 AND msg.id = $2 AND msg.version_id = $3 AND msg.deleted_at IS NULL
        "#, thread_id.into_inner(), message_id.into_inner(), version_id.into_inner()).fetch_one(&self.pool).await?;
        Ok(row.into())
    }

    async fn message_version_delete(
        &self,
        _thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        query!(
            "UPDATE message SET deleted_at = $3 WHERE id = $1 AND version_id = $2",
            message_id.into_inner(),
            version_id.into_inner(),
            now
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn message_version_list(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let items = query_as!(
            DbMessage,
            r#"
            with
            att_unnest as (select version_id, unnest(attachments) as media_id from message),
            att_json as (
                select version_id, json_agg(row_to_json(media)) as attachments
                from att_unnest
                join media on att_unnest.media_id = media.id
                group by att_unnest.version_id
            )
        select
            msg.type as "message_type: DbMessageType",
            msg.id,
            msg.thread_id, 
            msg.version_id,
            msg.ordering,
            msg.content,
            msg.metadata,
            msg.reply_id,
            msg.override_name,
            row_to_json(usr) as "author!: serde_json::Value",
            coalesce(att_json.attachments, '[]'::json) as "attachments!: serde_json::Value",
            false as "is_pinned!"
        from message as msg
        join usr on usr.id = msg.author_id
        left join att_json on att_json.version_id = msg.version_id
        where thread_id = $1 and msg.id = $2 and msg.deleted_at is null
          and msg.id > $3 AND msg.id < $4
		order by (CASE WHEN $5 = 'F' THEN msg.version_id END), msg.version_id DESC LIMIT $6
            "#,
            thread_id.into_inner(),
            message_id.into_inner(),
            p.after.into_inner(),
            p.before.into_inner(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            r#"
            select count(*) from message where thread_id = $1 and id = $2
            "#,
            thread_id.into_inner(),
            message_id.into_inner(),
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
}
