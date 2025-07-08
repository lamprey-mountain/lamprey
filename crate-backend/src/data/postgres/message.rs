use async_trait::async_trait;
use common::v1::types::{
    Embed, Interactions, Mentions, MessageDefaultMarkdown, MessageDefaultTagged,
    MessageThreadUpdate, MessageType, UserId,
};
use sqlx::{query, query_file_as, query_file_scalar, query_scalar, Acquire};
use tracing::info;
use uuid::Uuid;

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{
    DbMessageCreate, Message, MessageId, MessageVerId, PaginationDirection, PaginationQuery,
    PaginationResponse, ThreadId,
};

use crate::data::DataMessage;

use super::util::media_from_db;
use super::{Pagination, Postgres};

#[derive(Debug)]
pub struct DbMessage {
    pub message_type: DbMessageType,
    pub id: MessageId,
    pub thread_id: ThreadId,
    pub version_id: MessageVerId,
    pub ordering: i32,
    pub content: Option<String>,
    pub attachments: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<uuid::Uuid>,
    pub override_name: Option<String>, // temp?
    pub author_id: UserId,
    pub embeds: Option<serde_json::Value>,
    pub reactions: Option<serde_json::Value>,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "message_type")]
pub enum DbMessageType {
    DefaultMarkdown,
    DefaultTagged,
    ThreadUpdate,
}

impl From<MessageType> for DbMessageType {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::DefaultMarkdown(_) => DbMessageType::DefaultMarkdown,
            MessageType::DefaultTagged(_) => DbMessageType::DefaultTagged,
            MessageType::ThreadUpdate(_) => DbMessageType::ThreadUpdate,
            _ => todo!(),
        }
    }
}

impl From<DbMessage> for Message {
    fn from(row: DbMessage) -> Self {
        Message {
            id: row.id,
            message_type: match row.message_type {
                DbMessageType::DefaultMarkdown => {
                    let attachments: Vec<serde_json::Value> =
                        serde_json::from_value(row.attachments).unwrap_or_default();
                    let embeds: Vec<Embed> = row
                        .embeds
                        .and_then(|e| serde_json::from_value(e).ok())
                        .unwrap_or_default();
                    MessageType::DefaultMarkdown(MessageDefaultMarkdown {
                        content: row.content,
                        attachments: attachments.into_iter().map(media_from_db).collect(),
                        metadata: row.metadata,
                        reply_id: row.reply_id.map(Into::into),
                        override_name: row.override_name,
                        embeds,
                        reactions: row
                            .reactions
                            .map(|a| serde_json::from_value(a).unwrap())
                            .unwrap_or_default(),
                    })
                }
                DbMessageType::DefaultTagged => {
                    let attachments: Vec<serde_json::Value> =
                        serde_json::from_value(row.attachments).unwrap_or_default();
                    let embeds: Vec<Embed> = row
                        .embeds
                        .and_then(|e| serde_json::from_value(e).ok())
                        .unwrap_or_default();
                    MessageType::DefaultTagged(MessageDefaultTagged {
                        content: row.content,
                        attachments: attachments.into_iter().map(media_from_db).collect(),
                        metadata: row.metadata,
                        reply_id: row.reply_id.map(Into::into),
                        embeds,
                        reactions: row
                            .reactions
                            .map(|a| serde_json::from_value(a).unwrap())
                            .unwrap_or_default(),
                        interactions: Interactions::default(),
                    })
                }
                DbMessageType::ThreadUpdate => MessageType::ThreadUpdate(MessageThreadUpdate {
                    patch: row
                        .metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .unwrap_or_default(),
                }),
            },
            thread_id: row.thread_id,
            version_id: row.version_id,
            nonce: None,
            author_id: row.author_id,
            mentions: Mentions::default(),
            deleted_at: None,
        }
    }
}

#[async_trait]
impl DataMessage for Postgres {
    async fn message_create(&self, create: DbMessageCreate) -> Result<MessageId> {
        let message_id = Uuid::now_v7();
        let message_type: DbMessageType = create.message_type.clone().into();
        let mut tx = self.pool.begin().await?;
        let embeds = serde_json::to_value(create.embeds.clone())?;
        query!(r#"
    	    INSERT INTO message (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id, type, override_name, is_latest, embeds)
    	    VALUES ($1, $2, $3, (SELECT coalesce(max(ordering), 0) FROM message WHERE thread_id = $2), $4, $5, $6, $7, $8, $9, true, $10)
        "#,
            message_id,
            create.thread_id.into_inner(),
            message_id,
            create.content(),
            create.metadata(),
            create.reply_id().map(|i| i.into_inner()),
            create.author_id.into_inner(),
            message_type as _,
            create.override_name(),
            embeds,
        )
        .execute(&mut *tx)
        .await?;
        for (ord, att) in create.attachment_ids.iter().enumerate() {
            query!(
                r#"
        	    INSERT INTO message_attachment (version_id, media_id, ordering)
        	    VALUES ($1, $2, $3)
                "#,
                message_id,
                att.into_inner(),
                ord as i32
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        info!("insert message");
        Ok(message_id.into())
    }

    async fn message_update(
        &self,
        _thread_id: ThreadId,
        message_id: MessageId,
        create: DbMessageCreate,
    ) -> Result<MessageVerId> {
        let ver_id = Uuid::now_v7();
        let message_type: DbMessageType = create.message_type.clone().into();
        let mut tx = self.pool.begin().await?;
        query!(
            r#"UPDATE message SET is_latest = false WHERE id = $1"#,
            message_id.into_inner(),
        )
        .execute(&mut *tx)
        .await?;
        let embeds = serde_json::to_value(create.embeds.clone())?;
        query!(r#"
    	    INSERT INTO message (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id, type, override_name, is_latest, embeds)
    	    VALUES ($1, $2, $3, (SELECT coalesce(max(ordering), 0) FROM message WHERE thread_id = $2), $4, $5, $6, $7, $8, $9, true, $10)
        "#,
            message_id.into_inner(),
            create.thread_id.into_inner(),
            ver_id,
            create.content(),
            create.metadata(),
            create.reply_id().map(|i| i.into_inner()),
            create.author_id.into_inner(),
            message_type as _,
            create.override_name(),
            embeds,
        )
        .execute(&mut *tx)
        .await?;
        for (ord, att) in create.attachment_ids.iter().enumerate() {
            query!(
                r#"
        	    INSERT INTO message_attachment (version_id, media_id, ordering)
        	    VALUES ($1, $2, $3)
                "#,
                message_id.into_inner(),
                att.into_inner(),
                ord as i32
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        info!("update message");
        Ok(ver_id.into())
    }

    async fn message_get(&self, thread_id: ThreadId, id: MessageId) -> Result<Message> {
        let row = query_file_as!(
            DbMessage,
            "sql/message_get.sql",
            thread_id.into_inner(),
            id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn message_list(
        &self,
        thread_id: ThreadId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_paginate.sql",
                thread_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/message_count.sql", thread_id.into_inner())
        )
    }

    async fn message_delete(&self, _thread_id: ThreadId, message_id: MessageId) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        query!(
            "UPDATE message SET deleted_at = $2 WHERE id = $1",
            message_id.into_inner(),
            now,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn message_delete_bulk(
        &self,
        _thread_id: ThreadId,
        message_ids: &[MessageId],
    ) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let ids: Vec<Uuid> = message_ids.iter().map(|i| i.into_inner()).collect();
        query!(
            "UPDATE message SET deleted_at = $2 WHERE id = ANY($1)",
            &ids[..],
            now,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn message_version_get(
        &self,
        thread_id: ThreadId,
        version_id: MessageVerId,
    ) -> Result<Message> {
        let row = query_file_as!(
            DbMessage,
            "sql/message_version_get.sql",
            thread_id.into_inner(),
            version_id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn message_version_delete(
        &self,
        _thread_id: ThreadId,
        version_id: MessageVerId,
    ) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        query!(
            "UPDATE message SET deleted_at = $2 WHERE version_id = $1",
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
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                "sql/message_version_paginate.sql",
                thread_id.into_inner(),
                message_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r"select count(*) from message where thread_id = $1 and id = $2",
                thread_id.into_inner(),
                message_id.into_inner(),
            )
        )
    }
}
