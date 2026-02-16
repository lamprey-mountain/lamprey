use async_trait::async_trait;
use common::v1::types::reaction::ReactionCounts;
use common::v1::types::util::Time;
use common::v1::types::{
    ChannelType, Embed, Mentions, MessageDefaultMarkdown, MessageType, UserId,
};
use common::v2::types::message::{Message as MessageV2, MessageVersion as MessageVersionV2};
use sqlx::{query, query_file_as, query_file_scalar, query_scalar, Acquire};
use tracing::info;
use uuid::Uuid;

use crate::consts::MAX_PINNED_MESSAGES;
use crate::error::{Error, Result};
use crate::gen_paginate;
use crate::types::{
    ChannelId, DbChannelType, DbMessageCreate, MentionsIds, MessageId, MessageVerId,
    PaginationDirection, PaginationQuery, PaginationResponse,
};

use crate::data::DataMessage;

use super::util::media_from_db;
use super::{Pagination, Postgres};

#[derive(Debug)]
pub struct DbMessage {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub author_id: UserId,
    pub created_at: time::PrimitiveDateTime,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub removed_at: Option<time::PrimitiveDateTime>,
    pub pinned: Option<serde_json::Value>,
    pub message_type: DbMessageType,
    pub version_id: MessageVerId,
    pub version_author_id: UserId,
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<uuid::Uuid>,
    pub override_name: Option<String>, // temp?
    pub embeds: Option<serde_json::Value>,
    pub version_created_at: time::PrimitiveDateTime,
    pub version_deleted_at: Option<time::PrimitiveDateTime>,
    pub attachments: serde_json::Value,
}

#[derive(Debug)]
pub struct DbMessageVersion {
    pub version_id: MessageVerId,
    pub author_id: UserId,
    pub message_type: DbMessageType,
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<uuid::Uuid>,
    pub override_name: Option<String>,
    pub embeds: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub attachments: serde_json::Value,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "message_type")]
pub enum DbMessageType {
    DefaultMarkdown,
    DefaultTagged, // removed
    ThreadUpdate,  // removed
    MemberAdd,
    MemberRemove,
    MemberJoin,
    MessagePinned,
    ThreadCreated,
    ChannelRename,
    ChannelIcon,
    ChannelPingback,
    ChannelMoved,
    AutomodExecution,
    Call,
}

impl From<MessageType> for DbMessageType {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::DefaultMarkdown(_) => DbMessageType::DefaultMarkdown,
            MessageType::ChannelRename(_) => DbMessageType::ChannelRename,
            MessageType::MemberAdd(_) => DbMessageType::MemberAdd,
            MessageType::MemberRemove(_) => DbMessageType::MemberRemove,
            MessageType::MemberJoin => DbMessageType::MemberJoin,
            MessageType::Call(_) => DbMessageType::Call,
            MessageType::MessagePinned(_) => DbMessageType::MessagePinned,
            MessageType::ThreadCreated(_) => DbMessageType::ThreadCreated,
            MessageType::ChannelIcon(_) => DbMessageType::ChannelIcon,
            MessageType::ChannelPingback(_) => DbMessageType::ChannelPingback,
            MessageType::ChannelMoved(_) => DbMessageType::ChannelMoved,
            MessageType::AutomodExecution(_) => DbMessageType::AutomodExecution,
        }
    }
}

impl From<DbMessage> for MessageV2 {
    fn from(row: DbMessage) -> Self {
        MessageV2 {
            id: row.id,
            channel_id: row.channel_id,
            author_id: row.author_id,
            created_at: Time::from(row.created_at),
            deleted_at: row.deleted_at.map(Time::from),
            removed_at: row.removed_at.map(Time::from),
            pinned: row.pinned.and_then(|p| serde_json::from_value(p).ok()),
            reactions: ReactionCounts(vec![]),
            latest_version: DbMessageVersion {
                version_id: row.version_id,
                author_id: row.version_author_id,
                message_type: row.message_type,
                content: row.content,
                metadata: row.metadata,
                reply_id: row.reply_id,
                override_name: row.override_name,
                embeds: row.embeds,
                created_at: row.version_created_at,
                deleted_at: row.version_deleted_at,
                attachments: row.attachments,
            }
            .into(),
            thread: None,
        }
    }
}

impl From<DbMessageVersion> for MessageVersionV2 {
    fn from(row: DbMessageVersion) -> Self {
        MessageVersionV2 {
            version_id: row.version_id,
            author_id: Some(row.author_id),
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
                DbMessageType::ChannelRename => MessageType::ChannelRename(
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
                DbMessageType::ThreadCreated => MessageType::ThreadCreated(
                    row.metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .expect("invalid data in db"),
                ),
                DbMessageType::ChannelIcon => MessageType::ChannelIcon(
                    row.metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .expect("invalid data in db"),
                ),
                DbMessageType::AutomodExecution => MessageType::AutomodExecution(
                    row.metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .expect("invalid data in db"),
                ),
                DbMessageType::ChannelPingback => MessageType::ChannelPingback(
                    row.metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .expect("invalid data in db"),
                ),
                DbMessageType::ChannelMoved => MessageType::ChannelMoved(
                    row.metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .expect("invalid data in db"),
                ),
                DbMessageType::Call => MessageType::Call(
                    row.metadata
                        .and_then(|m| serde_json::from_value(m).ok())
                        .expect("invalid data in db"),
                ),
                ty @ DbMessageType::ThreadUpdate | ty @ DbMessageType::DefaultTagged => {
                    panic!("{ty:?} messages are deprecated and shouldn't exist in the database anymore")
                }
            },
            mentions: Mentions::default(),
            created_at: Time::from(row.created_at),
            deleted_at: row.deleted_at.map(Time::from),
        }
    }
}

#[async_trait]
impl DataMessage for Postgres {
    async fn message_create(&self, create: DbMessageCreate) -> Result<MessageId> {
        let message_id = create
            .id
            .map(|i| i.into_inner())
            .unwrap_or_else(Uuid::now_v7);
        // the version_id of the first version of a message is the same as the message id itself
        let version_id = message_id;
        let message_type: DbMessageType = create.message_type.clone().into();
        let mut tx = self.pool.begin().await?;

        let channel_type: ChannelType = query_scalar!(
            r#"SELECT type as "type: DbChannelType" FROM channel WHERE id = $1"#,
            *create.channel_id
        )
        .fetch_one(&mut *tx)
        .await?
        .into();

        if channel_type.is_thread() {
            query!(
                "UPDATE channel SET last_activity_at = NOW() WHERE id = $1",
                *create.channel_id
            )
            .execute(&mut *tx)
            .await?;
        }

        let embeds = create.embeds.clone();
        let embeds_json = serde_json::to_value(&embeds)?;
        let mentions: MentionsIds = create.mentions.clone().into();
        let mentions_json = serde_json::to_value(mentions)?;
        let created_at = create
            .created_at
            .map(|t| t.assume_utc())
            .unwrap_or_else(time::OffsetDateTime::now_utc);
        let created_at = time::PrimitiveDateTime::new(created_at.date(), created_at.time());

        let removed_at = create.removed_at.map(|t| t.assume_utc());
        let removed_at = removed_at.map(|t| time::PrimitiveDateTime::new(t.date(), t.time()));

        query!(
            r#"INSERT INTO message (id, channel_id, author_id, created_at, removed_at, latest_version_id)
            VALUES ($1, $2, $3, $4, $5, $6)"#,
            message_id,
            *create.channel_id,
            create.author_id.into_inner(),
            created_at,
            removed_at,
            version_id,
        )
        .execute(&mut *tx)
        .await?;

        query!(
            r#"INSERT INTO message_version (version_id, message_id, author_id, type, content, metadata, reply_id, mentions, embeds, created_at, override_name)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
            version_id,
            message_id,
            create.author_id.into_inner(),
            message_type as _,
            create.content(),
            create.metadata(),
            create.reply_id().map(|i| i.into_inner()),
            mentions_json,
            embeds_json,
            created_at,
            create.override_name(),
        )
        .execute(&mut *tx)
        .await?;

        for (ord, att) in create.attachment_ids.iter().enumerate() {
            query!(
                r#"
                INSERT INTO message_attachment (version_id, media_id, ordering)
                VALUES ($1, $2, $3)
                "#,
                version_id,
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
        _channel_id: ChannelId,
        message_id: MessageId,
        create: DbMessageCreate,
    ) -> Result<MessageVerId> {
        let ver_id = Uuid::now_v7();
        let message_type: DbMessageType = create.message_type.clone().into();
        let mut tx = self.pool.begin().await?;

        let embeds = create.embeds.clone();
        let embeds_json = serde_json::to_value(&embeds)?;
        let mentions: MentionsIds = create.mentions.clone().into();
        let mentions_json = serde_json::to_value(mentions)?;
        let created_at = create
            .edited_at
            .map(|t| t.assume_utc())
            .unwrap_or_else(time::OffsetDateTime::now_utc);
        let created_at = time::PrimitiveDateTime::new(created_at.date(), created_at.time());

        query!(
            r#"UPDATE message SET latest_version_id = $1 WHERE id = $2"#,
            ver_id,
            *message_id,
        )
        .execute(&mut *tx)
        .await?;

        query!(
            r#"INSERT INTO message_version (version_id, message_id, author_id, type, content, metadata, reply_id, mentions, embeds, created_at, override_name)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
            ver_id,
            *message_id,
            create.author_id.into_inner(),
            message_type as _,
            create.content(),
            create.metadata(),
            create.reply_id().map(|i| i.into_inner()),
            mentions_json,
            embeds_json,
            created_at,
            create.override_name(),
        )
        .execute(&mut *tx)
        .await?;

        for (ord, att) in create.attachment_ids.iter().enumerate() {
            query!(
                r#"
                INSERT INTO message_attachment (version_id, media_id, ordering)
                VALUES ($1, $2, $3)
                "#,
                ver_id,
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

    // NOTE: ignores channel_id, attachment_ids in create
    async fn message_update_in_place(
        &self,
        _channel_id: ChannelId,
        version_id: MessageVerId,
        create: DbMessageCreate,
    ) -> Result<()> {
        let message_type: DbMessageType = create.message_type.clone().into();
        let mut tx = self.pool.begin().await?;
        let embeds = create.embeds.clone();
        let embeds_json = serde_json::to_value(&embeds)?;
        let mentions: MentionsIds = create.mentions.clone().into();
        let mentions_json = serde_json::to_value(mentions)?;
        let created_at = create.edited_at.map(|t| t.assume_utc());
        let created_at = created_at.map(|t| time::PrimitiveDateTime::new(t.date(), t.time()));

        query!(
            r#"
            UPDATE message_version SET
                content = $2,
                metadata = $3,
                reply_id = $4,
                author_id = $5,
                type = $6,
                override_name = $7,
                embeds = $8,
                mentions = $9,
                created_at = $10
            WHERE version_id = $1
        "#,
            *version_id,
            create.content(),
            create.metadata(),
            create.reply_id().map(|i| i.into_inner()),
            create.author_id.into_inner(),
            message_type as _,
            create.override_name(),
            embeds_json,
            mentions_json,
            created_at,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        info!("update message in place");
        Ok(())
    }

    async fn message_get(
        &self,
        channel_id: ChannelId,
        id: MessageId,
        _user_id: UserId,
    ) -> Result<MessageV2> {
        let row = query_file_as!(DbMessage, "sql/message_get.sql", *channel_id, *id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.into())
    }

    async fn message_get_many(
        &self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
        _user_id: UserId,
    ) -> Result<Vec<MessageV2>> {
        let ids: Vec<Uuid> = message_ids.iter().map(|id| **id).collect();
        let rows = query_file_as!(DbMessage, "sql/message_get_many.sql", *channel_id, &ids)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn message_list(
        &self,
        channel_id: ChannelId,
        _user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_paginate.sql",
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/message_count.sql", channel_id.into_inner()),
            |i: &MessageV2| i.id.to_string()
        )
    }

    async fn message_list_deleted(
        &self,
        channel_id: ChannelId,
        _user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_paginate_deleted.sql",
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/message_count_deleted.sql", channel_id.into_inner()),
            |i: &MessageV2| i.id.to_string()
        )
    }

    async fn message_list_removed(
        &self,
        channel_id: ChannelId,
        _user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_paginate_removed.sql",
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/message_count_removed.sql", channel_id.into_inner()),
            |i: &MessageV2| i.id.to_string()
        )
    }

    async fn message_list_activity(
        &self,
        channel_id: ChannelId,
        _user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_activity_paginate.sql",
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/message_activity_count.sql", channel_id.into_inner()),
            |i: &MessageV2| i.id.to_string()
        )
    }

    async fn message_delete(&self, _channel_id: ChannelId, message_id: MessageId) -> Result<()> {
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
        _channel_id: ChannelId,
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
        _channel_id: ChannelId,
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
        _channel_id: ChannelId,
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
        channel_id: ChannelId,
        version_id: MessageVerId,
        _user_id: UserId,
    ) -> Result<MessageVersionV2> {
        let row = query_file_as!(
            DbMessageVersion,
            "sql/message_version_get.sql",
            *channel_id,
            *version_id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn message_version_delete(
        &self,
        _channel_id: ChannelId,
        version_id: MessageVerId,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let now = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());
        let version_uuid = version_id.into_inner();

        query!(
            r#"
            UPDATE message_version
            SET
                deleted_at = $2,
                content = NULL,
                embeds = '[]'::jsonb
            WHERE version_id = $1
            "#,
            version_uuid,
            now
        )
        .execute(&mut *tx)
        .await?;

        query!(
            "DELETE FROM message_attachment WHERE version_id = $1",
            version_uuid
        )
        .execute(&mut *tx)
        .await?;

        query!(
            "DELETE FROM media_link WHERE target_id = $1 AND link_type = 'MessageVersion'",
            version_uuid
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn message_version_list(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        _user_id: UserId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<MessageVersionV2>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessageVersion,
                "sql/message_version_paginate.sql",
                *channel_id,
                *message_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r"select count(*) from message where channel_id = $1 and id = $2",
                channel_id.into_inner(),
                message_id.into_inner(),
            ),
            |i: &MessageVersionV2| i.version_id.to_string()
        )
    }

    async fn message_replies(
        &self,
        channel_id: ChannelId,
        root_message_id: Option<MessageId>,
        _user_id: UserId,
        depth: u16,
        breadth: Option<u16>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>> {
        let p: Pagination<_> = pagination.try_into()?;
        let rmid = root_message_id.map(|i| *i);
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_replies.sql",
                *channel_id,
                rmid,
                depth as i32,
                breadth.map(|b| b as i64),
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
            ),
            query_file_scalar!(
                "sql/message_replies_count.sql",
                *channel_id,
                rmid,
                depth as i32,
                breadth.map(|b| b as i64)
            ),
            |i: &MessageV2| i.id.to_string()
        )
    }

    async fn message_pin_create(&self, channel_id: ChannelId, message_id: MessageId) -> Result<()> {
        let pin_count: i64 = query_scalar!(
            "select count(*) from message where channel_id = $1 and pinned is not null",
            *channel_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or_default();

        if pin_count >= MAX_PINNED_MESSAGES as i64 {
            return Err(Error::BadStatic("too many pins"));
        }

        let mut tx = self.pool.begin().await?;

        query!(
            "update message set pinned = jsonb_set(pinned, '{position}', ((pinned->>'position')::int + 1)::text::jsonb) where channel_id = $1 and pinned is not null",
            *channel_id
        )
        .execute(&mut *tx)
        .await?;

        let pinned = serde_json::json!({
            "time": Time::now_utc(),
            "position": 0,
        });

        query!(
            "update message set pinned = $1 where id = $2 and channel_id = $3",
            pinned,
            *message_id,
            *channel_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    async fn message_pin_delete(&self, channel_id: ChannelId, message_id: MessageId) -> Result<()> {
        query!(
            "update message set pinned = null where id = $1 and channel_id = $2",
            *message_id,
            *channel_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn message_pin_reorder(
        &self,
        channel_id: ChannelId,
        reorder: common::v1::types::PinsReorder,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for item in reorder.messages {
            if let Some(Some(pos)) = item.position {
                let old_pinned: Option<serde_json::Value> = query_scalar!(
                    "select pinned from message where id = $1 and channel_id = $2",
                    *item.id,
                    *channel_id
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
                    "update message set pinned = $1 where id = $2 and channel_id = $3",
                    pinned,
                    *item.id,
                    *channel_id
                )
                .execute(&mut *tx)
                .await?;
            } else if let Some(None) = item.position {
                // unpin
                query!(
                    "update message set pinned = null where id = $1 and channel_id = $2",
                    *item.id,
                    *channel_id
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
        channel_id: ChannelId,
        _user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_pin_list.sql",
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/message_pin_list_count.sql", *channel_id),
            |i: &MessageV2| i.id.to_string()
        )
    }

    async fn message_get_ancestors(
        &self,
        message_id: MessageId,
        limit: u16,
    ) -> Result<Vec<MessageV2>> {
        let rows = query_file_as!(
            DbMessage,
            "sql/message_get_ancestors.sql",
            *message_id,
            limit as i32
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn message_fetch_mention_ids(
        &self,
        _channel_id: ChannelId,
        version_ids: &[MessageVerId],
    ) -> Result<Vec<MentionsIds>> {
        let version_uuids: Vec<Uuid> = version_ids.iter().map(|id| **id).collect();

        let rows = query!(
            r#"
            SELECT mentions
            FROM message_version
            WHERE version_id = ANY($1)
            "#,
            &version_uuids[..]
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            if let Some(mentions_json) = row.mentions {
                let mentions: MentionsIds = serde_json::from_value(mentions_json)?;
                result.push(mentions);
            } else {
                result.push(MentionsIds::default());
            }
        }

        Ok(result)
    }

    async fn message_list_all(
        &self,
        channel_id: ChannelId,
        _user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<MessageV2>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_list_all.sql",
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/message_list_all_count.sql", channel_id.into_inner()),
            |i: &MessageV2| i.id.to_string()
        )
    }

    async fn message_id_get_by_version(
        &self,
        channel_id: ChannelId,
        version_id: MessageVerId,
    ) -> Result<MessageId> {
        let message_id = query_scalar!(
            r#"
            SELECT m.id
            FROM message_version mv
            JOIN message m ON m.id = mv.message_id
            WHERE mv.version_id = $1 AND m.channel_id = $2
            "#,
            *version_id,
            *channel_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(message_id.into())
    }
}
