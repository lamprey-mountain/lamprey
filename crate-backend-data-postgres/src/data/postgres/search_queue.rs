use async_trait::async_trait;
use sqlx::query;
use uuid::Uuid;

use crate::{data::DataSearchQueue, error::Result, types::RoomId};

use super::Postgres;

#[async_trait]
impl DataSearchQueue for Postgres {
    async fn search_reindex_queue_upsert(
        &self,
        target_type: &str,
        target_id: Uuid,
        last_id: Option<Uuid>,
    ) -> Result<()> {
        query!(
            "INSERT INTO search_reindex_queue (target_id, target_type, last_id) VALUES ($1, $2, $3) ON CONFLICT (target_id, target_type) DO UPDATE SET last_id = $3, updated_at = NOW()",
            target_id,
            target_type,
            last_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn search_reindex_queue_list(
        &self,
        target_type: &str,
        limit: u32,
    ) -> Result<Vec<(Uuid, Option<Uuid>)>> {
        let rows = query!(
            r#"SELECT target_id, last_id FROM search_reindex_queue WHERE target_type = $1 ORDER BY updated_at ASC LIMIT $2"#,
            target_type,
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.target_id, r.last_id)).collect())
    }

    async fn search_reindex_queue_delete(&self, target_type: &str, target_id: Uuid) -> Result<()> {
        query!(
            "DELETE FROM search_reindex_queue WHERE target_id = $1 AND target_type = $2",
            target_id,
            target_type
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn search_reindex_queue_get(
        &self,
        target_type: &str,
        target_id: Uuid,
    ) -> Result<Option<Uuid>> {
        let row = query!(
            r#"SELECT last_id FROM search_reindex_queue WHERE target_id = $1 AND target_type = $2"#,
            target_id,
            target_type
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|r| r.last_id))
    }

    async fn search_reindex_queue_upsert_room(&self, room_id: RoomId) -> Result<()> {
        query!(
            r#"INSERT INTO search_reindex_queue (target_id, target_type) SELECT id, 'channel' FROM channel WHERE room_id = $1 AND deleted_at IS NULL AND archived_at IS NULL ON CONFLICT (target_id, target_type) DO NOTHING"#,
            *room_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn search_reindex_queue_upsert_all(&self) -> Result<()> {
        query!(
            r#"INSERT INTO search_reindex_queue (target_id, target_type) SELECT id, 'channel' FROM channel WHERE deleted_at IS NULL AND archived_at IS NULL ON CONFLICT (target_id, target_type) DO NOTHING"#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
