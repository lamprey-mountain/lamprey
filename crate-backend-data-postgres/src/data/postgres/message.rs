// TODO: remove `user_id` params

use std::collections::HashMap;

use async_trait::async_trait;
use common::v1::types::components::{self, Components};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::message::{
    Message, MessageAttachment, MessageAttachmentType, MessageDefaultMarkdown, MessageInteraction,
    MessageType, MessageVersion,
};
use common::v1::types::reaction::{ReactionCounts, ReactionKey};
use common::v1::types::sync::ChannelSync;
use common::v1::types::util::Time;
use common::v1::types::{ChannelSeq, ChannelType, Mentions, UserId};
use common::v2::types::embed::Embed;
use sqlx::{query, query_as, query_file_as, query_file_scalar, query_scalar};
use tracing::info;
use uuid::Uuid;

use crate::consts::MAX_PINNED_MESSAGES;
use crate::data::postgres::DbMediaData;
use crate::error::{Error, Result};
use crate::gen_paginate;
use crate::types::{
    ChannelId, DbChannelType, DbMessageCreate, DbMessageExtract, DbMessageType, DbMessageUpdate,
    MentionsIds, MessageId, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse,
};

use crate::data::DataMessage;

use super::{Pagination, Postgres};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbMessage {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub room_id: Option<Uuid>,
    pub author_id: Uuid,
    pub created_at: time::PrimitiveDateTime,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub removed_at: Option<time::PrimitiveDateTime>,
    pub pinned: Option<serde_json::Value>,
    pub message_type: DbMessageType,
    pub version_id: Uuid,
    pub version_author_id: Uuid,
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<uuid::Uuid>,
    pub override_name: Option<String>, // temp?
    pub embeds: Option<serde_json::Value>,
    pub components: Option<serde_json::Value>,
    pub version_created_at: time::PrimitiveDateTime,
    pub version_deleted_at: Option<time::PrimitiveDateTime>,
    pub attachments: serde_json::Value,
    pub created_seq: i64,
    pub version_created_seq: i64,
    pub lifecycle_seq: i64,
    pub flume: Option<serde_json::Value>,
    pub interaction: Option<serde_json::Value>,
    pub ephemeral: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DbMessageVersion {
    pub version_id: Uuid,
    pub author_id: Uuid,
    pub message_type: DbMessageType,
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub reply_id: Option<uuid::Uuid>,
    pub override_name: Option<String>,
    pub embeds: Option<serde_json::Value>,
    pub components: Option<serde_json::Value>,
    pub created_at: time::PrimitiveDateTime,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub attachments: serde_json::Value,
    pub created_seq: i64,
}

impl From<DbMessage> for Message {
    fn from(row: DbMessage) -> Self {
        Message {
            id: row.id.into(),
            channel_id: row.channel_id.into(),
            room_id: row.room_id.map(|i| i.into()),
            author_id: row.author_id.into(),
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
                components: row.components,
                created_at: row.version_created_at,
                deleted_at: row.version_deleted_at,
                attachments: row.attachments,
                created_seq: row.version_created_seq,
            }
            .into(),
            thread: None,
            flume: row.flume.and_then(|v| serde_json::from_value(v).ok()),
            interaction: row.interaction.and_then(|v| serde_json::from_value(v).ok()),
            ephemeral: row.ephemeral,
        }
    }
}

impl From<DbMessageVersion> for MessageVersion {
    fn from(row: DbMessageVersion) -> Self {
        MessageVersion {
            version_id: row.version_id.into(),
            author_id: Some(row.author_id.into()),
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
                        attachments: attachments
                            .into_iter()
                            .map(|v| {
                                let media: DbMediaData = serde_json::from_value(v)
                                    .unwrap_or_else(|_| panic!("invalid attachment"));
                                MessageAttachment {
                                    ty: MessageAttachmentType::Media {
                                        media: media.into(),
                                    },
                                    spoiler: false,
                                }
                            })
                            .collect(),
                        metadata: row.metadata.and_then(|m| serde_json::from_value(m).ok()),
                        reply_id: row.reply_id.map(Into::into),
                        embeds,
                        // NOTE: actual components are populated in the messages service
                        components: Components::default(),
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
    async fn message_create(&mut self, create: DbMessageCreate) -> Result<MessageId> {
        let message_id = create
            .id
            .map(|i| i.into_inner())
            .unwrap_or_else(Uuid::now_v7);
        // the version_id of the first version of a message is the same as the message id itself
        let version_id = message_id;
        let message_type: DbMessageType = create.message_type.clone().into();
        let mut tx = self.begin_tx().await?;

        let channel_type: ChannelType = query_scalar!(
            r#"SELECT type as "type: DbChannelType" FROM channel WHERE id = $1"#,
            *create.channel_id
        )
        .fetch_one(tx.ext())
        .await?
        .into();

        if channel_type.is_thread() {
            query!(
                "UPDATE channel SET last_activity_at = NOW() WHERE id = $1",
                *create.channel_id
            )
            .execute(tx.ext())
            .await?;
        }

        let embeds = create.embeds.clone();
        let embeds_json = serde_json::to_value(&embeds)?;
        let components = create.components.clone();
        let components_json = serde_json::to_value(&components)?;
        let mentions: MentionsIds = create.mentions.clone().into();
        let mentions_json = serde_json::to_value(mentions)?;
        let created_at = create
            .created_at
            .map(|t| t.assume_utc())
            .unwrap_or_else(time::OffsetDateTime::now_utc);
        let created_at = time::PrimitiveDateTime::new(created_at.date(), created_at.time());

        let removed_at = create.removed_at.map(|t| t.assume_utc());
        let removed_at = removed_at.map(|t| time::PrimitiveDateTime::new(t.date(), t.time()));

        // Atomically increment the channel's latest_seq and get the new value
        let new_seq: i64 = query_scalar!(
            r#"UPDATE channel SET latest_seq = latest_seq + 1 WHERE id = $1 RETURNING latest_seq as "latest_seq!""#,
            *create.channel_id
        )
        .fetch_one(tx.ext())
        .await?;

        let flume_json = create.flume.clone();
        let interaction_json = create.interaction.clone();
        let ephemeral = create.ephemeral;
        query!(
            r#"INSERT INTO message (id, channel_id, author_id, created_at, removed_at, latest_version_id, created_seq, lifecycle_seq, flume, interaction, ephemeral)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $7, $8, $9, $10)"#,
            message_id,
            *create.channel_id,
            create.author_id.into_inner(),
            created_at,
            removed_at,
            version_id,
            new_seq,
            flume_json,
            interaction_json,
            ephemeral,
        )
        .execute(tx.ext())
        .await?;

        query!(
            "UPDATE channel SET last_message_id = $1, last_version_id = $2 WHERE id = $3",
            message_id,
            version_id,
            *create.channel_id
        )
        .execute(tx.ext())
        .await?;

        query!(
            r#"INSERT INTO message_version (version_id, message_id, author_id, type, content, metadata, reply_id, mentions, embeds, created_at, override_name, created_seq, components)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
            version_id,
            message_id,
            create.author_id.into_inner(),
            message_type as _,
            create.message_type.content(),
            create.message_type.metadata(),
            create.message_type.reply_id().map(|i| i.into_inner()),
            mentions_json,
            embeds_json,
            created_at,
            create.message_type.override_name(),
            new_seq,
            components_json,
        )
        .execute(tx.ext())
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
            .execute(tx.ext())
            .await?;
        }
        tx.commit().await?;
        info!("insert message");
        Ok(message_id.into())
    }

    async fn message_update(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
        update: DbMessageUpdate,
    ) -> Result<MessageVerId> {
        let ver_id = Uuid::now_v7();
        let message_type: DbMessageType = update.message_type.clone().into();
        let mut tx = self.begin_tx().await?;

        let embeds = update.embeds.clone();
        let embeds_json = serde_json::to_value(&embeds)?;
        let components = update.components.clone();
        let components_json = serde_json::to_value(&components)?;
        let mentions: MentionsIds = update.mentions.clone().into();
        let mentions_json = serde_json::to_value(mentions)?;
        let created_at = update
            .created_at
            .map(|t| t.assume_utc())
            .unwrap_or_else(time::OffsetDateTime::now_utc);
        let created_at = time::PrimitiveDateTime::new(created_at.date(), created_at.time());

        // Atomically increment the channel's latest_seq and get the new value
        let new_seq: i64 = query_scalar!(
            r#"UPDATE channel SET latest_seq = latest_seq + 1 WHERE id = $1 RETURNING latest_seq as "latest_seq!""#,
            *channel_id
        )
        .fetch_one(tx.ext())
        .await?;

        // Also bump the message's version
        query!(
            r#"UPDATE message SET latest_version_id = $1 WHERE id = $2"#,
            ver_id,
            *message_id,
        )
        .execute(tx.ext())
        .await?;

        query!(
            r#"INSERT INTO message_version (version_id, message_id, author_id, type, content, metadata, reply_id, mentions, embeds, created_at, override_name, created_seq, components)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
            ver_id,
            *message_id,
            update.author_id.into_inner(),
            message_type as _,
            update.message_type.content(),
            update.message_type.metadata(),
            update.message_type.reply_id().map(|i| i.into_inner()),
            mentions_json,
            embeds_json,
            created_at,
            update.message_type.override_name(),
            new_seq,
            components_json,
        )
        .execute(tx.ext())
        .await?;

        for (ord, att) in update.attachment_ids.iter().enumerate() {
            query!(
                r#"
                INSERT INTO message_attachment (version_id, media_id, ordering)
                VALUES ($1, $2, $3)
                "#,
                ver_id,
                att.into_inner(),
                ord as i32
            )
            .execute(tx.ext())
            .await?;
        }

        query!(
            "UPDATE channel SET last_version_id = $1 WHERE id = $2",
            ver_id,
            *channel_id
        )
        .execute(tx.ext())
        .await?;

        tx.commit().await?;
        info!("update message");
        Ok(ver_id.into())
    }

    // NOTE: ignores channel_id, attachment_ids in update
    async fn message_update_in_place(
        &mut self,
        _channel_id: ChannelId,
        version_id: MessageVerId,
        update: DbMessageUpdate,
    ) -> Result<()> {
        let message_type: DbMessageType = update.message_type.clone().into();
        let mut tx = self.begin_tx().await?;
        let embeds = update.embeds.clone();
        let embeds_json = serde_json::to_value(&embeds)?;
        let components = update.components.clone();
        let components_json = serde_json::to_value(&components)?;
        let mentions: MentionsIds = update.mentions.clone().into();
        let mentions_json = serde_json::to_value(mentions)?;
        let created_at = update.created_at.map(|t| t.assume_utc());
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
                created_at = $10,
                components = $11
            WHERE version_id = $1
        "#,
            *version_id,
            update.message_type.content(),
            update.message_type.metadata(),
            update.message_type.reply_id().map(|i| i.into_inner()),
            update.author_id.into_inner(),
            message_type as _,
            update.message_type.override_name(),
            embeds_json,
            mentions_json,
            created_at,
            components_json,
        )
        .execute(tx.ext())
        .await?;
        tx.commit().await?;
        info!("update message in place");
        Ok(())
    }

    async fn message_flume_update(
        &mut self,
        message_id: MessageId,
        flume: serde_json::Value,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"UPDATE message SET flume = $1 WHERE id = $2"#,
            flume,
            *message_id,
        )
        .execute(conn.ext())
        .await?;
        info!("update message flume");
        Ok(())
    }

    async fn message_get(&mut self, channel_id: ChannelId, id: MessageId) -> Result<Message> {
        let mut conn = self.acquire().await?;
        let row = query_file_as!(DbMessage, "sql/message_get.sql", *channel_id, *id)
            .fetch_one(conn.ext())
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    Error::ApiError(ApiError::from_code(ErrorCode::UnknownMessage))
                }
                e => Error::Sqlx(e),
            })?;
        Ok(row.into())
    }

    async fn message_get_many(
        &mut self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<Vec<Message>> {
        let mut conn = self.acquire().await?;
        let ids: Vec<Uuid> = message_ids.iter().map(|id| **id).collect();
        let rows = query_file_as!(DbMessage, "sql/message_get_many.sql", *channel_id, &ids)
            .fetch_all(conn.ext())
            .await?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn message_list(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
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
            |i: &Message| i.id.to_string()
        )
    }

    async fn message_list_deleted(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
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
            |i: &Message| i.id.to_string()
        )
    }

    async fn message_list_removed(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
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
            |i: &Message| i.id.to_string()
        )
    }

    async fn message_list_activity(
        &mut self,
        channel_id: ChannelId,
        _user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
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
            |i: &Message| i.id.to_string()
        )
    }

    async fn message_delete(&mut self, channel_id: ChannelId, message_id: MessageId) -> Result<()> {
        let mut tx = self.begin_tx().await?;
        let now = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());

        // Atomically increment seq
        let new_seq: i64 = query_scalar!(
            r#"UPDATE channel SET latest_seq = latest_seq + 1 WHERE id = $1 RETURNING latest_seq as "latest_seq!""#,
            *channel_id
        )
        .fetch_one(tx.ext())
        .await?;

        query!(
            "UPDATE message SET deleted_at = $2, lifecycle_seq = $3 WHERE id = $1 AND deleted_at IS NULL",
            message_id.into_inner(),
            now,
            new_seq,
        )
        .execute(tx.ext())
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn message_delete_bulk(
        &mut self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<()> {
        let mut tx = self.begin_tx().await?;
        let now = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());
        let ids: Vec<Uuid> = message_ids.iter().map(|i| i.into_inner()).collect();

        // Atomically increment seq
        let new_seq: i64 = query_scalar!(
            r#"UPDATE channel SET latest_seq = latest_seq + 1 WHERE id = $1 RETURNING latest_seq as "latest_seq!""#,
            *channel_id
        )
        .fetch_one(tx.ext())
        .await?;

        query!(
            "UPDATE message SET deleted_at = $2, lifecycle_seq = $3 WHERE id = ANY($1) AND deleted_at IS NULL",
            &ids[..],
            now,
            new_seq,
        )
        .execute(tx.ext())
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn message_remove_bulk(
        &mut self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<()> {
        let mut tx = self.begin_tx().await?;
        let now = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());
        let ids: Vec<Uuid> = message_ids.iter().map(|i| i.into_inner()).collect();

        // Atomically increment seq
        let new_seq: i64 = query_scalar!(
            r#"UPDATE channel SET latest_seq = latest_seq + 1 WHERE id = $1 RETURNING latest_seq as "latest_seq!""#,
            *channel_id
        )
        .fetch_one(tx.ext())
        .await?;

        query!(
            "UPDATE message SET removed_at = $2, lifecycle_seq = $3 WHERE id = ANY($1) AND removed_at IS NULL",
            &ids[..],
            now,
            new_seq,
        )
        .execute(tx.ext())
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn message_restore_bulk(
        &mut self,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<()> {
        let mut tx = self.begin_tx().await?;
        let ids: Vec<Uuid> = message_ids.iter().map(|i| i.into_inner()).collect();

        // Atomically increment seq
        let new_seq: i64 = query_scalar!(
            r#"UPDATE channel SET latest_seq = latest_seq + 1 WHERE id = $1 RETURNING latest_seq as "latest_seq!""#,
            *channel_id
        )
        .fetch_one(tx.ext())
        .await?;

        query!(
            "UPDATE message SET removed_at = NULL, lifecycle_seq = $2 WHERE id = ANY($1) AND removed_at IS NOT NULL",
            &ids[..],
            new_seq,
        )
        .execute(tx.ext())
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn message_version_get(
        &mut self,
        channel_id: ChannelId,
        version_id: MessageVerId,
    ) -> Result<MessageVersion> {
        let mut conn = self.acquire().await?;
        let row = query_file_as!(
            DbMessageVersion,
            "sql/message_version_get.sql",
            *channel_id,
            *version_id,
        )
        .fetch_one(conn.ext())
        .await?;
        Ok(row.into())
    }

    async fn message_version_delete(
        &mut self,
        channel_id: ChannelId,
        version_id: MessageVerId,
    ) -> Result<()> {
        let mut tx = self.begin_tx().await?;
        let now = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());
        let version_uuid = version_id.into_inner();

        // Atomically increment seq
        let new_seq: i64 = query_scalar!(
            r#"UPDATE channel SET latest_seq = latest_seq + 1 WHERE id = $1 RETURNING latest_seq as "latest_seq!""#,
            *channel_id
        )
        .fetch_one(tx.ext())
        .await?;

        query!(
            r#"
            UPDATE message_version
            SET
                deleted_at = $2,
                content = NULL,
                embeds = '[]'::jsonb,
                created_seq = $3
            WHERE version_id = $1
            "#,
            version_uuid,
            now,
            new_seq,
        )
        .execute(tx.ext())
        .await?;

        query!(
            "DELETE FROM message_attachment WHERE version_id = $1",
            version_uuid
        )
        .execute(tx.ext())
        .await?;

        query!(
            "DELETE FROM media_link WHERE target_id = $1 AND link_type = 'MessageVersion'",
            version_uuid
        )
        .execute(tx.ext())
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn message_version_list(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<MessageVersion>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
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
            |i: &MessageVersion| i.version_id.to_string()
        )
    }

    async fn message_replies(
        &mut self,
        channel_id: ChannelId,
        root_message_id: Option<MessageId>,
        _user_id: UserId,
        depth: u16,
        breadth: Option<u16>,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        let rmid = root_message_id.map(|i| *i);
        gen_paginate!(
            p,
            self,
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
            |i: &Message| i.id.to_string()
        )
    }

    async fn message_pin_create(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<bool> {
        let mut conn = self.acquire().await?;
        let pinned: Option<serde_json::Value> = query_scalar!(
            "select pinned from message where id = $1 and channel_id = $2",
            *message_id,
            *channel_id
        )
        .fetch_optional(conn.ext())
        .await?
        .flatten();

        if pinned.is_some() {
            return Ok(false);
        }

        let pin_count: i64 = query_scalar!(
            "select count(*) from message where channel_id = $1 and pinned is not null",
            *channel_id
        )
        .fetch_one(conn.ext())
        .await?
        .unwrap_or_default();

        if pin_count >= MAX_PINNED_MESSAGES as i64 {
            return Err(Error::BadStatic("too many pins"));
        }

        let mut tx = self.begin_tx().await?;

        query!(
            "update message set pinned = jsonb_set(pinned, '{position}', ((pinned->>'position')::int + 1)::text::jsonb) where channel_id = $1 and pinned is not null",
            *channel_id
        )
        .execute(tx.ext())
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
        .execute(tx.ext())
        .await?;

        tx.commit().await?;

        Ok(true)
    }

    async fn message_pin_delete(
        &mut self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "update message set pinned = null where id = $1 and channel_id = $2",
            *message_id,
            *channel_id
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn message_pin_reorder(
        &mut self,
        channel_id: ChannelId,
        reorder: common::v1::types::PinsReorder,
    ) -> Result<()> {
        let mut tx = self.begin_tx().await?;
        for item in reorder.messages {
            if let Some(Some(pos)) = item.position {
                let old_pinned: Option<serde_json::Value> = query_scalar!(
                    "select pinned from message where id = $1 and channel_id = $2",
                    *item.id,
                    *channel_id
                )
                .fetch_one(tx.ext())
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
                .execute(tx.ext())
                .await?;
            } else if let Some(None) = item.position {
                // unpin
                query!(
                    "update message set pinned = null where id = $1 and channel_id = $2",
                    *item.id,
                    *channel_id
                )
                .execute(tx.ext())
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn message_pin_list(
        &mut self,
        channel_id: ChannelId,
        _user_id: UserId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
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
            |i: &Message| i.id.to_string()
        )
    }

    async fn message_get_ancestors(
        &mut self,
        message_id: MessageId,
        limit: u16,
    ) -> Result<Vec<Message>> {
        let mut conn = self.acquire().await?;
        let rows = query_file_as!(
            DbMessage,
            "sql/message_get_ancestors.sql",
            *message_id,
            limit as i32
        )
        .fetch_all(conn.ext())
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn message_fetch_mention_ids(
        &mut self,
        _channel_id: ChannelId,
        version_ids: &[MessageVerId],
    ) -> Result<Vec<MentionsIds>> {
        let mut conn = self.acquire().await?;
        let version_uuids: Vec<Uuid> = version_ids.iter().map(|id| **id).collect();

        let rows = query!(
            r#"
            SELECT mentions
            FROM message_version
            WHERE version_id = ANY($1)
            "#,
            &version_uuids[..]
        )
        .fetch_all(conn.ext())
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

    async fn message_fetch_components(
        &mut self,
        channel_id: ChannelId,
        version_ids: &[MessageVerId],
    ) -> Result<HashMap<MessageVerId, Components<components::Thin>>> {
        let mut conn = self.acquire().await?;
        let version_uuids: Vec<Uuid> = version_ids.iter().map(|id| **id).collect();

        let rows = query!(
            r#"
            SELECT components, mv.version_id
            FROM message_version AS mv
            JOIN message AS m ON mv.message_id = m.id
            WHERE mv.version_id = ANY($1) AND m.channel_id = $2
            "#,
            &version_uuids[..],
            *channel_id
        )
        .fetch_all(conn.ext())
        .await?;

        let result: HashMap<MessageVerId, _> = rows
            .into_iter()
            .filter_map(|r| {
                if let Some(c) = r.components {
                    let c: Components<components::Thin> = serde_json::from_value(c).ok()?;
                    Some((r.version_id.into(), c))
                } else {
                    None
                }
            })
            .collect();

        Ok(result)
    }

    async fn message_list_all(
        &mut self,
        channel_id: ChannelId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self,
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
            |i: &Message| i.id.to_string()
        )
    }

    async fn message_id_get_by_version(
        &mut self,
        channel_id: ChannelId,
        version_id: MessageVerId,
    ) -> Result<MessageId> {
        let mut conn = self.acquire().await?;
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
        .fetch_one(conn.ext())
        .await?;
        Ok(message_id.into())
    }

    async fn channel_sync(
        &mut self,
        channel_id: ChannelId,
        since: ChannelSeq,
        pagination: PaginationQuery<MessageId>,
        _user_id: Option<UserId>,
    ) -> Result<ChannelSync> {
        use common::v1::types::emoji::EmojiCustom;
        use common::v1::types::reaction::ReactionKeyParam;
        use common::v1::types::MessageSync;
        use std::str::FromStr;

        let mut conn = self.acquire().await?;
        let p: Pagination<_> = pagination.try_into()?;
        let limit = p.limit;

        // Snapshot the channel's current latest_seq upfront to avoid a race
        // where the trailing read observes a seq that events were not returned for.
        let channel_latest_seq: i64 = query_scalar!(
            r#"SELECT latest_seq as "latest_seq!" FROM channel WHERE id = $1"#,
            *channel_id
        )
        .fetch_one(conn.ext())
        .await?;

        // Bound by the exact number of allowed distinctive `seq` sequences. (Prevents row limitations
        // from splitting bulk events that all share the exact same sequence integer)
        let cutoff_seq: Option<i64> = query_scalar!(
            r#"
            WITH seqs AS (
                SELECT created_seq as seq FROM message WHERE channel_id = $1 AND created_seq > $2
                UNION
                SELECT lifecycle_seq as seq FROM message WHERE channel_id = $1 AND lifecycle_seq > $2 AND lifecycle_seq != created_seq
                UNION
                SELECT mv.created_seq as seq FROM message_version mv JOIN message m ON mv.message_id = m.id WHERE m.channel_id = $1 AND mv.created_seq > $2 AND mv.created_seq != m.created_seq
                UNION
                SELECT created_seq as seq FROM reaction WHERE channel_id = $1 AND created_seq > $2
                UNION
                SELECT deleted_seq as seq FROM reaction WHERE channel_id = $1 AND deleted_seq IS NOT NULL AND deleted_seq > $2
                ORDER BY seq ASC LIMIT $3
            )
            SELECT MAX(seq) FROM seqs
            "#,
            *channel_id,
            since.0 as i64,
            (limit + 1) as i32
        )
        .fetch_one(conn.ext())
        .await?;

        let Some(max_seq) = cutoff_seq else {
            return Ok(ChannelSync {
                events: vec![],
                seq: ChannelSeq(channel_latest_seq as u64),
                partial: false,
            });
        };

        // Fetch message creates since the given seq
        let messages: Vec<DbMessage> = query_as!(
            DbMessage,
            r#"
            SELECT
                mv.type as "message_type: DbMessageType",
                m.id,
                m.channel_id,
                c.room_id,
                m.author_id,
                m.created_at,
                m.deleted_at,
                m.removed_at,
                m.pinned,
                mv.version_id,
                mv.author_id as version_author_id,
                mv.content,
                mv.metadata,
                mv.reply_id,
                mv.override_name,
                mv.embeds as "embeds",
                mv.components as "components",
                mv.created_at as "version_created_at",
                mv.deleted_at as "version_deleted_at",
                coalesce(att_json.attachments, '[]'::json) as "attachments!",
                m.created_seq,
                mv.created_seq as "version_created_seq",
                m.lifecycle_seq,
                m.flume,
                m.interaction,
                m.ephemeral
            FROM message AS m
            JOIN message_version AS mv ON m.latest_version_id = mv.version_id
            LEFT JOIN att_json ON att_json.version_id = mv.version_id
            JOIN channel AS c ON m.channel_id = c.id
            WHERE m.channel_id = $1
              AND m.created_seq > $2
              AND m.created_seq <= $3
            ORDER BY m.created_seq ASC, m.id ASC
            "#,
            *channel_id,
            since.0 as i64,
            max_seq
        )
        .fetch_all(conn.ext())
        .await?;

        // Fetch message lifecycle events (delete/remove/restore) since the given seq
        let lifecycle_events = query!(
            r#"
            SELECT id as "id: Uuid", lifecycle_seq, deleted_at, removed_at
            FROM message
            WHERE channel_id = $1
              AND lifecycle_seq > $2
              AND lifecycle_seq <= $3
              AND lifecycle_seq != created_seq
            ORDER BY lifecycle_seq ASC, id ASC
            "#,
            *channel_id,
            since.0 as i64,
            max_seq
        )
        .fetch_all(conn.ext())
        .await?;

        // Fetch message versions that changed since the given seq (edits, version deletes)
        let changed_versions = query!(
            r#"
            SELECT mv.version_id, mv.message_id, mv.created_seq, mv.deleted_at
            FROM message_version mv
            JOIN message m ON mv.message_id = m.id
            WHERE m.channel_id = $1
              AND mv.created_seq > $2
              AND mv.created_seq <= $3
              AND mv.created_seq != m.created_seq
            ORDER BY mv.created_seq ASC, mv.version_id ASC
            "#,
            *channel_id,
            since.0 as i64,
            max_seq
        )
        .fetch_all(conn.ext())
        .await?;

        // Batch fetch full messages for non-deleted changed versions
        let version_message_ids: Vec<Uuid> = changed_versions
            .iter()
            .filter(|v| v.deleted_at.is_none())
            .map(|v| v.message_id)
            .collect();
        let version_messages: Vec<DbMessage> = if version_message_ids.is_empty() {
            vec![]
        } else {
            query_file_as!(
                DbMessage,
                "sql/message_get_many.sql",
                *channel_id,
                &version_message_ids[..]
            )
            .fetch_all(conn.ext())
            .await?
        };
        let version_message_map: std::collections::HashMap<Uuid, DbMessage> =
            version_messages.into_iter().map(|m| (m.id, m)).collect();

        // Fetch reaction creates since the given seq, with emoji data
        let reaction_creates = query!(
            r#"
            SELECT
                r.message_id as "message_id: Uuid",
                r.user_id as "user_id: Uuid",
                r.key,
                r.created_at,
                e.id as "emoji_id?",
                e.name as "emoji_name?",
                e.animated as "emoji_animated?",
                e.media_id as "emoji_media_id?",
                r.created_seq as "seq!"
            FROM reaction r
            LEFT JOIN custom_emoji e ON (
                CASE WHEN r.key LIKE 'c:%' THEN
                    SUBSTRING(r.key FROM 3)::uuid = e.id
                ELSE false END
            )
            WHERE r.channel_id = $1
            AND r.created_seq > $2
            AND r.created_seq <= $3
            ORDER BY r.created_seq ASC
            "#,
            *channel_id,
            since.0 as i64,
            max_seq
        )
        .fetch_all(conn.ext())
        .await?;

        // Fetch reaction deletes since the given seq, with emoji data
        let reaction_deletes = query!(
            r#"
            SELECT
                r.message_id as "message_id: Uuid",
                r.user_id as "user_id: Uuid",
                r.key,
                e.id as "emoji_id?",
                e.name as "emoji_name?",
                e.animated as "emoji_animated?",
                e.media_id as "emoji_media_id?",
                r.deleted_seq as "seq!"
            FROM reaction r
            LEFT JOIN custom_emoji e ON (
                CASE WHEN r.key LIKE 'c:%' THEN
                    SUBSTRING(r.key FROM 3)::uuid = e.id
                ELSE false END
            )
            WHERE r.channel_id = $1
            AND r.deleted_seq IS NOT NULL
            AND r.deleted_seq > $2
            AND r.deleted_seq <= $3
            ORDER BY r.deleted_seq ASC
            "#,
            *channel_id,
            since.0 as i64,
            max_seq
        )
        .fetch_all(conn.ext())
        .await?;

        // Helper to build ReactionKey from key string + optional emoji data.
        // Returns None if the emoji is unknown (for custom reactions).
        let make_reaction_key = |key_str: String,
                                 emoji_id: Option<Uuid>,
                                 emoji_name: Option<String>,
                                 emoji_animated: Option<bool>,
                                 emoji_media_id: Option<Uuid>|
         -> Option<ReactionKey> {
            let key = match ReactionKeyParam::from_str(&key_str) {
                Ok(k) => k,
                Err(()) => return None,
            };
            match key {
                ReactionKeyParam::Text(content) => Some(ReactionKey::Text { content }),
                ReactionKeyParam::Custom(id) => {
                    if let (Some(name), Some(media)) = (emoji_name, emoji_media_id) {
                        Some(ReactionKey::Custom(EmojiCustom {
                            id: emoji_id.map(|e| e.into()).unwrap_or(id),
                            name,
                            creator_id: None,
                            owner: None,
                            animated: emoji_animated.unwrap_or(false),
                            media_id: media.into(),
                        }))
                    } else {
                        // Emoji is unknown - log and filter out
                        tracing::warn!(
                            emoji_id = %id,
                            "skipping reaction for unknown custom emoji during sync"
                        );
                        None
                    }
                }
            }
        };

        // Combine all events and sort by seq
        struct InternalEvent {
            seq: i64,
            event: MessageSync,
        }

        let mut all_events = Vec::new();

        // Group messages by lifecycle_seq for bulk delete/remove/restore
        let mut delete_groups: std::collections::HashMap<i64, Vec<MessageId>> =
            std::collections::HashMap::new();
        let mut remove_groups: std::collections::HashMap<i64, Vec<MessageId>> =
            std::collections::HashMap::new();
        let mut restore_groups: std::collections::HashMap<i64, Vec<MessageId>> =
            std::collections::HashMap::new();

        // Message creates: use created_seq (not lifecycle_seq)
        for msg in messages {
            let seq = msg.created_seq;
            let message = Message::from(msg);
            all_events.push(InternalEvent {
                seq,
                event: MessageSync::MessageCreate { message },
            });
        }

        // Lifecycle events (delete/remove/restore): use lifecycle_seq
        for msg in lifecycle_events {
            let seq = msg.lifecycle_seq;
            let msg_id: MessageId = msg.id.into();

            if msg.deleted_at.is_some() {
                delete_groups.entry(seq).or_default().push(msg_id);
            } else if msg.removed_at.is_some() {
                remove_groups.entry(seq).or_default().push(msg_id);
            } else {
                restore_groups.entry(seq).or_default().push(msg_id);
            }
        }

        // Emit bulk delete/remove/restore events
        for (seq, message_ids) in delete_groups {
            if message_ids.len() == 1 {
                all_events.push(InternalEvent {
                    seq,
                    event: MessageSync::MessageDelete {
                        channel_id,
                        message_id: message_ids[0],
                    },
                });
            } else {
                all_events.push(InternalEvent {
                    seq,
                    event: MessageSync::MessageDeleteBulk {
                        channel_id,
                        message_ids,
                    },
                });
            }
        }

        for (seq, message_ids) in remove_groups {
            all_events.push(InternalEvent {
                seq,
                event: MessageSync::MessageRemove {
                    channel_id,
                    message_ids,
                },
            });
        }

        for (seq, message_ids) in restore_groups {
            all_events.push(InternalEvent {
                seq,
                event: MessageSync::MessageRestore {
                    channel_id,
                    message_ids,
                },
            });
        }

        // Process changed versions
        for ver in changed_versions {
            if ver.deleted_at.is_some() {
                all_events.push(InternalEvent {
                    seq: ver.created_seq,
                    event: MessageSync::MessageVersionDelete {
                        channel_id,
                        message_id: ver.message_id.into(),
                        version_id: ver.version_id.into(),
                    },
                });
            } else if let Some(msg) = version_message_map.get(&ver.message_id) {
                all_events.push(InternalEvent {
                    seq: ver.created_seq,
                    event: MessageSync::MessageUpdate {
                        message: Message::from(msg.clone()),
                    },
                });
            }
        }

        // Process reaction creates, skipping unknown emoji
        for r in reaction_creates {
            if let Some(key) = make_reaction_key(
                r.key,
                r.emoji_id,
                r.emoji_name,
                r.emoji_animated,
                r.emoji_media_id,
            ) {
                all_events.push(InternalEvent {
                    seq: r.seq,
                    event: MessageSync::ReactionCreate {
                        user_id: r.user_id.into(),
                        channel_id,
                        message_id: r.message_id.into(),
                        key,
                    },
                });
            }
        }

        // Process reaction deletes, skipping unknown emoji
        for r in reaction_deletes {
            if let Some(key) = make_reaction_key(
                r.key,
                r.emoji_id,
                r.emoji_name,
                r.emoji_animated,
                r.emoji_media_id,
            ) {
                all_events.push(InternalEvent {
                    seq: r.seq,
                    event: MessageSync::ReactionDelete {
                        user_id: r.user_id.into(),
                        channel_id,
                        message_id: r.message_id.into(),
                        key,
                    },
                });
            }
        }

        // Sort by seq
        all_events.sort_by_key(|e| e.seq);

        // Count distinct seq groups and truncate at the limit boundary
        let mut distinct_seqs = 0;
        let mut last_seq = None;
        let mut cutoff_idx = None;

        for (i, event) in all_events.iter().enumerate() {
            if last_seq != Some(event.seq) {
                if distinct_seqs == limit as usize {
                    cutoff_idx = Some(i);
                    break;
                }
                distinct_seqs += 1;
                last_seq = Some(event.seq);
            }
        }

        let partial = cutoff_idx.is_some();

        if let Some(cutoff) = cutoff_idx {
            all_events.truncate(cutoff);
        }

        // When partial, return the seq of the last fully-returned event group.
        // This ensures co-seq events aren't lost on the next call.
        let seq = if partial {
            ChannelSeq(all_events.last().map(|e| e.seq as u64).unwrap_or(since.0))
        } else {
            let last_event_seq = all_events
                .last()
                .map(|e| e.seq as i64)
                .unwrap_or(since.0 as i64);
            ChannelSeq(std::cmp::max(channel_latest_seq, last_event_seq) as u64)
        };

        let events = all_events.into_iter().map(|e| e.event).collect();

        Ok(ChannelSync {
            events,
            seq,
            partial,
        })
    }
}
