use async_trait::async_trait;
use lamprey_backend_core::types::admin::DlqEntry;
use sqlx::query;
use uuid::Uuid;

use common::v1::types::{PaginationQuery, PaginationResponse, SearchDlqId};

use crate::{data::DataSearchQueue, error::Result, types::RoomId};

use super::Postgres;

#[async_trait]
impl DataSearchQueue for Postgres {
    async fn search_reindex_queue_upsert(
        &mut self,
        target_type: &str,
        target_id: Uuid,
        last_id: Option<Uuid>,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "INSERT INTO search_reindex_queue (target_id, target_type, last_id) VALUES ($1, $2, $3) ON CONFLICT (target_id, target_type) DO UPDATE SET last_id = $3, updated_at = NOW()",
            target_id,
            target_type,
            last_id,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn search_reindex_queue_list(
        &mut self,
        target_type: &str,
        limit: u32,
    ) -> Result<Vec<(Uuid, Option<Uuid>)>> {
        let mut conn = self.acquire().await?;
        let rows = query!(
            r#"SELECT target_id, last_id FROM search_reindex_queue WHERE target_type = $1 ORDER BY updated_at ASC LIMIT $2"#,
            target_type,
            limit as i64
        )
        .fetch_all(conn.ext())
        .await?;
        Ok(rows.into_iter().map(|r| (r.target_id, r.last_id)).collect())
    }

    async fn search_reindex_queue_delete(
        &mut self,
        target_type: &str,
        target_id: Uuid,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "DELETE FROM search_reindex_queue WHERE target_id = $1 AND target_type = $2",
            target_id,
            target_type
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn search_reindex_queue_get(
        &mut self,
        target_type: &str,
        target_id: Uuid,
    ) -> Result<Option<Uuid>> {
        let mut conn = self.acquire().await?;
        let row = query!(
            r#"SELECT last_id FROM search_reindex_queue WHERE target_id = $1 AND target_type = $2"#,
            target_id,
            target_type
        )
        .fetch_optional(conn.ext())
        .await?;
        Ok(row.and_then(|r| r.last_id))
    }

    async fn search_reindex_queue_upsert_room(&mut self, room_id: RoomId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"INSERT INTO search_reindex_queue (target_id, target_type) SELECT id, 'channel' FROM channel WHERE room_id = $1 AND deleted_at IS NULL AND archived_at IS NULL ON CONFLICT (target_id, target_type) DO NOTHING"#,
            *room_id,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn search_reindex_queue_upsert_all(&mut self) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"INSERT INTO search_reindex_queue (target_id, target_type) SELECT id, 'channel' FROM channel WHERE deleted_at IS NULL AND archived_at IS NULL ON CONFLICT (target_id, target_type) DO NOTHING"#,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn search_ingestion_dlq_insert(
        &mut self,
        entity_id: Uuid,
        entity_type: &str,
        error_message: &str,
    ) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "INSERT INTO search_ingestion_dlq (id, entity_id, entity_type, error_message) VALUES ($1, $2, $3, $4)",
            Uuid::now_v7(),
            entity_id,
            entity_type,
            error_message
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn search_ingestion_dlq_list(
        &mut self,
        pagination: PaginationQuery<SearchDlqId>,
    ) -> Result<PaginationResponse<DlqEntry>> {
        let mut conn = self.acquire().await?;
        let items = query!(
            r#"SELECT id, entity_id, entity_type, error_message, created_at FROM search_ingestion_dlq WHERE id > $1 ORDER BY id ASC LIMIT $2"#,
            *pagination.from.unwrap_or_default(),
            pagination.limit.unwrap_or(100) as i64
        )
        .fetch_all(conn.ext())
        .await?
        .into_iter()
        .map(|r| DlqEntry {
            id: r.id.into(),
            entity_id: r.entity_id,
            entity_type: r.entity_type,
            error_message: r.error_message,
            created_at: r.created_at.into(),
        })
        .collect::<Vec<_>>();

        let has_more = items.len() as u16 >= pagination.limit.unwrap_or(100);

        Ok(PaginationResponse {
            items,
            has_more,
            total: 0, // TODO: implement count if needed
            cursor: None,
        })
    }

    async fn search_ingestion_dlq_delete(&mut self, id: SearchDlqId) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!("DELETE FROM search_ingestion_dlq WHERE id = $1", *id)
            .execute(conn.ext())
            .await?;
        Ok(())
    }
}
