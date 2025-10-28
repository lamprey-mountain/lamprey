use async_trait::async_trait;
use common::v1::types::{
    misc::Color,
    tag::{Tag, TagCreate, TagPatch},
    ChannelId, TagId,
};
use sqlx::{query, query_as};
use uuid::Uuid;

use crate::{data::DataTag, error::Result};

use super::Postgres;

struct DbTag {
    id: Uuid,
    name: String,
    description: Option<String>,
    color: Option<String>,
    is_archived: bool,
    is_restricted: bool,
    active_thread_count: i64,
    total_thread_count: i64,
}

impl From<DbTag> for Tag {
    fn from(tag: DbTag) -> Self {
        Self {
            id: tag.id.into(),
            name: tag.name,
            description: tag.description,
            color: tag.color.map(Color::Srgb),
            archived: tag.is_archived,
            restricted: tag.is_restricted,
            active_thread_count: tag.active_thread_count as u64,
            total_thread_count: tag.total_thread_count as u64,
        }
    }
}

#[async_trait]
impl DataTag for Postgres {
    async fn tag_create(&self, forum_channel_id: ChannelId, create: TagCreate) -> Result<Tag> {
        let tag_id = TagId::new();
        let mut tx = self.pool.begin().await?;

        let color = create.color.map(|c| c.as_ref().to_string());

        let tag = query_as!(
            DbTag,
            r#"
            WITH t AS (
                INSERT INTO tag (id, version_id, channel_id, name, description, color, is_archived, is_restricted)
                VALUES ($1, $1, $2, $3, $4, $5, false, $6)
                RETURNING id, name, description, color, is_archived, is_restricted
            )
            SELECT t.*, 0 as "active_thread_count!", 0 as "total_thread_count!" FROM t
            "#,
            *tag_id,
            *forum_channel_id,
            create.name,
            create.description,
            color,
            create.restricted,
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(tag.into())
    }

    async fn tag_update(&self, tag_id: TagId, patch: TagPatch) -> Result<Tag> {
        let mut tx = self.pool.begin().await?;

        let old_tag = self.tag_get(tag_id).await?;

        let color = patch.color.map(|c| c.map(|c| c.as_ref().to_string()));

        let tag = query_as!(
            DbTag,
            r#"
            WITH t AS (
                UPDATE tag
                SET
                    name = $2,
                    description = $3,
                    color = $4,
                    is_archived = $5,
                    is_restricted = $6
                WHERE id = $1
                RETURNING id, name, description, color, is_archived, is_restricted
            ),
            active_threads AS (
                SELECT count(*) FROM channel_tag ct JOIN channel c ON ct.channel_id = c.id WHERE ct.tag_id = $1 AND c.archived_at IS NULL
            ),
            total_threads AS (
                SELECT count(*) FROM channel_tag WHERE tag_id = $1
            )
            SELECT t.*, (SELECT count FROM active_threads) as "active_thread_count!", (SELECT count FROM total_threads) as "total_thread_count!" FROM t
            "#,
            *tag_id,
            patch.name.unwrap_or(old_tag.name),
            patch.description.unwrap_or(old_tag.description),
            color.unwrap_or(old_tag.color.map(|c| c.as_ref().to_string())),
            patch.archived.unwrap_or(old_tag.archived),
            patch.restricted.unwrap_or(old_tag.restricted),
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(tag.into())
    }

    async fn tag_delete(&self, tag_id: TagId) -> Result<()> {
        query!("DELETE FROM tag WHERE id = $1", *tag_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn tag_get(&self, tag_id: TagId) -> Result<Tag> {
        let tag = query_as!(
            DbTag,
            r#"
            WITH active_threads AS (
                SELECT count(*) FROM channel_tag ct JOIN channel c ON ct.channel_id = c.id WHERE ct.tag_id = $1 AND c.archived_at IS NULL
            ),
            total_threads AS (
                SELECT count(*) FROM channel_tag WHERE tag_id = $1
            )
            SELECT
                t.id, t.name, t.description, t.color, t.is_archived, t.is_restricted,
                (SELECT count FROM active_threads) as "active_thread_count!",
                (SELECT count FROM total_threads) as "total_thread_count!"
            FROM tag t
            WHERE t.id = $1
            "#,
            *tag_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(tag.into())
    }

    async fn tag_get_forum_id(&self, tag_id: TagId) -> Result<ChannelId> {
        let forum_id = sqlx::query_scalar!("SELECT channel_id FROM tag WHERE id = $1", *tag_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(forum_id.into())
    }
}
