use async_trait::async_trait;
use common::v1::types::util::Time;
use common::v1::types::{Embed, MessageDefaultMarkdown, MessageType, UserId};
use sqlx::{query, query_file_as, query_file_scalar, query_scalar, Acquire};
use tracing::info;
use uuid::Uuid;

use crate::consts::MAX_PINNED_MESSAGES;
use crate::error::{Error, Result};
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
    pub created_at: Option<time::PrimitiveDateTime>,
    pub edited_at: Option<time::PrimitiveDateTime>,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub removed_at: Option<time::PrimitiveDateTime>,
    pub pinned: Option<serde_json::Value>,
    pub mentions: Option<serde_json::Value>,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "message_type")]
pub enum DbMessageType {
    DefaultMarkdown,
    DefaultTagged, // removed
    ThreadUpdate,  // removed
    ThreadRename,
    MemberAdd,
    MemberRemove,
    MemberJoin,
    MessagePinned,
}

impl From<MessageType> for DbMessageType {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::DefaultMarkdown(_) => DbMessageType::DefaultMarkdown,
            MessageType::ThreadRename(_) => DbMessageType::ThreadRename,
            MessageType::MemberAdd(_) => DbMessageType::MemberAdd,
            MessageType::MemberRemove(_) => DbMessageType::MemberRemove,
            MessageType::MemberJoin => DbMessageType::MemberJoin,
            MessageType::MessagePinned(_) => DbMessageType::MessagePinned,
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
                    })
                }
                DbMessageType::ThreadRename => MessageType::ThreadRename(
                    row.metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .expect("invalid data in db"),
                ),
                DbMessageType::MemberAdd => MessageType::MemberAdd(
                    row.metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .expect("invalid data in db"),
                ),
                DbMessageType::MemberRemove => MessageType::MemberRemove(
                    row.metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .expect("invalid data in db"),
                ),
                DbMessageType::MemberJoin => MessageType::MemberJoin,
                DbMessageType::MessagePinned => MessageType::MessagePinned(
                    row.metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .expect("invalid data in db"),
                ),
                ty => {
                    panic!("{ty:?} messages are deprecated and shouldn't exist in the database anymore")
                }
            },
            thread_id: row.thread_id,
            version_id: row.version_id,
            nonce: None,
            author_id: row.author_id,
            deleted_at: row.deleted_at.map(Time::from),
            edited_at: row.edited_at.map(Time::from),
            created_at: row.created_at.map(Time::from),
            removed_at: row.removed_at.map(Time::from),
            pinned: row.pinned.and_then(|p| serde_json::from_value(p).ok()),
            reactions: row
                .reactions
                .map(|a| serde_json::from_value(a).unwrap())
                .unwrap_or_default(),
            mentions: row
                .mentions
                .map(|a| serde_json::from_value(a).unwrap())
                .unwrap_or_default(),
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
        let mentions = serde_json::to_value(create.mentions.clone())?;
        query!(r#"
    	    INSERT INTO message (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id, type, override_name, is_latest, embeds, created_at, mentions)
    	    VALUES ($1, $2, $3, (SELECT coalesce(max(ordering), 0) FROM message WHERE thread_id = $2), $4, $5, $6, $7, $8, $9, true, $10, coalesce($11, now()), $12)
        "#,
            message_id,
            *create.thread_id,
            message_id,
            create.content(),
            create.metadata(),
            create.reply_id().map(|i| i.into_inner()),
            create.author_id.into_inner(),
            message_type as _,
            create.override_name(),
            embeds,
            create.created_at.map(|t| t.assume_utc()),
            mentions,
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
    	    INSERT INTO message (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id, type, override_name, is_latest, embeds, created_at, edited_at)
    	    VALUES ($1, $2, $3, (SELECT coalesce(max(ordering), 0) FROM message WHERE thread_id = $2), $4, $5, $6, $7, $8, $9, true, $10, $11, coalesce($12, now()))
        "#,
            *message_id,
            *create.thread_id,
            ver_id,
            create.content(),
            create.metadata(),
            create.reply_id().map(|i| *i),
            *create.author_id,
            message_type as _,
            create.override_name(),
            embeds,
            create.created_at,
            create.edited_at.map(|t| t.assume_utc()),
        )
        .execute(&mut *tx)
        .await?;
        for (ord, att) in create.attachment_ids.iter().enumerate() {
            query!(
                r#"
        	    INSERT INTO message_attachment (version_id, media_id, ordering)
        	    VALUES ($1, $2, $3)
                "#,
                *message_id,
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

    // NOTE: ignores thread_id, attachment_ids in create
    async fn message_update_in_place(
        &self,
        _thread_id: ThreadId,
        version_id: MessageVerId,
        create: DbMessageCreate,
    ) -> Result<()> {
        let message_type: DbMessageType = create.message_type.clone().into();
        let mut tx = self.pool.begin().await?;
        let embeds = serde_json::to_value(create.embeds.clone())?;
        query!(
            r#"
            UPDATE message SET
                content = $2,
                metadata = $3,
                reply_id = $4,
                author_id = $5,
                type = $6,
                override_name = $7,
                embeds = $8,
                created_at = $9,
                edited_at = $10
            WHERE version_id = $1
        "#,
            *version_id,
            create.content(),
            create.metadata(),
            create.reply_id().map(|i| *i),
            *create.author_id,
            message_type as _,
            create.override_name(),
            embeds,
            create.created_at,
            create.edited_at,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        info!("update message in place");
        Ok(())
    }

    async fn message_get(
        &self,
        thread_id: ThreadId,
        id: MessageId,
        user_id: UserId,
    ) -> Result<Message> {
        let row = query_file_as!(DbMessage, "sql/message_get.sql", *thread_id, *id, *user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.into())
    }

    async fn message_list(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_paginate.sql",
                *thread_id,
                *user_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/message_count.sql", thread_id.into_inner()),
            |i: &Message| i.id.to_string()
        )
    }

    async fn message_delete(&self, _thread_id: ThreadId, message_id: MessageId) -> Result<()> {
        let now = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());
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
        let now = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());
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

    async fn message_remove_bulk(
        &self,
        _thread_id: ThreadId,
        message_ids: &[MessageId],
    ) -> Result<()> {
        let now = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());
        let ids: Vec<Uuid> = message_ids.iter().map(|i| i.into_inner()).collect();
        query!(
            "UPDATE message SET removed_at = $2 WHERE id = ANY($1)",
            &ids[..],
            now,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn message_restore_bulk(
        &self,
        _thread_id: ThreadId,
        message_ids: &[MessageId],
    ) -> Result<()> {
        let ids: Vec<Uuid> = message_ids.iter().map(|i| i.into_inner()).collect();
        query!(
            "UPDATE message SET removed_at = NULL WHERE id = ANY($1)",
            &ids[..],
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn message_version_get(
        &self,
        thread_id: ThreadId,
        version_id: MessageVerId,
        user_id: UserId,
    ) -> Result<Message> {
        let row = query_file_as!(
            DbMessage,
            "sql/message_version_get.sql",
            *thread_id,
            *user_id,
            *version_id,
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
        let now = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());
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
        user_id: UserId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                "sql/message_version_paginate.sql",
                *thread_id,
                *message_id,
                *user_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r"select count(*) from message where thread_id = $1 and id = $2",
                thread_id.into_inner(),
                message_id.into_inner(),
            ),
            |i: &Message| i.version_id.to_string()
        )
    }

    async fn message_replies(
        &self,
        thread_id: ThreadId,
        root_message_id: Option<MessageId>,
        user_id: UserId,
        depth: u16,
        breadth: Option<u16>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        let rmid = root_message_id.map(|i| *i);
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_replies.sql",
                *thread_id,
                rmid,
                depth as i32,
                breadth.map(|b| b as i64),
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
                *user_id,
            ),
            query_file_scalar!(
                "sql/message_replies_count.sql",
                *thread_id,
                rmid,
                depth as i32
            ),
            |i: &Message| i.id.to_string()
        )
    }

    async fn message_pin_create(&self, thread_id: ThreadId, message_id: MessageId) -> Result<()> {
        let pin_count: i64 = query_scalar!(
            "select count(*) from message where thread_id = $1 and pinned is not null",
            *thread_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or_default();

        if pin_count >= MAX_PINNED_MESSAGES as i64 {
            return Err(Error::BadStatic("too many pins"));
        }

        let mut tx = self.pool.begin().await?;

        query!(
            "update message set pinned = jsonb_set(pinned, '{position}', ((pinned->>'position')::int + 1)::text::jsonb) where thread_id = $1 and pinned is not null",
            *thread_id
        )
        .execute(&mut *tx)
        .await?;

        let pinned = serde_json::json!({
            "time": Time::now_utc(),
            "position": 0,
        });

        query!(
            "update message set pinned = $1 where id = $2 and thread_id = $3",
            pinned,
            *message_id,
            *thread_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    async fn message_pin_delete(&self, thread_id: ThreadId, message_id: MessageId) -> Result<()> {
        query!(
            "update message set pinned = null where id = $1 and thread_id = $2",
            *message_id,
            *thread_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn message_pin_reorder(
        &self,
        thread_id: ThreadId,
        reorder: common::v1::types::PinsReorder,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for item in reorder.messages {
            if let Some(Some(pos)) = item.position {
                let old_pinned: Option<serde_json::Value> = query_scalar!(
                    "select pinned from message where id = $1 and thread_id = $2",
                    *item.id,
                    *thread_id
                )
                .fetch_one(&mut *tx)
                .await?;

                let time = if let Some(p) = old_pinned {
                    p.get("time")
                        .cloned()
                        .unwrap_or_else(|| serde_json::to_value(Time::now_utc()).unwrap())
                } else {
                    serde_json::to_value(Time::now_utc()).unwrap()
                };

                let pinned = serde_json::json!({
                    "time": time,
                    "position": pos,
                });
                query!(
                    "update message set pinned = $1 where id = $2 and thread_id = $3",
                    pinned,
                    *item.id,
                    *thread_id
                )
                .execute(&mut *tx)
                .await?;
            } else if let Some(None) = item.position {
                // unpin
                query!(
                    "update message set pinned = null where id = $1 and thread_id = $2",
                    *item.id,
                    *thread_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn message_pin_list(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_pin_list.sql",
                *thread_id,
                *user_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/message_pin_list_count.sql", *thread_id),
            |i: &Message| i.id.to_string()
        )
    }
}
