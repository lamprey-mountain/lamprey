use async_trait::async_trait;
use common::v1::types::RoomId;
use lamprey_backend_core::types::{
    admin::DlqEntry,
    data::{SearchReindexQueue, SearchReindexQueueTarget},
};
use sqlx::query;
use uuid::Uuid;

use common::v1::types::{PaginationQuery, PaginationResponse, SearchDlqId};

use crate::{data::DataSearchQueue, error::Result};

use super::Postgres;

#[async_trait]
impl DataSearchQueue for Postgres {
    async fn search_reindex_queue_upsert(
        &mut self,
        target: SearchReindexQueueTarget,
        last_id: Option<Uuid>,
    ) -> Result<()> {
        let (target_id, target_type) = match target {
            SearchReindexQueueTarget::Messages(id) => (*id, "messages"),
            SearchReindexQueueTarget::Channels => (Uuid::nil(), "channels"),
            SearchReindexQueueTarget::Rooms => (Uuid::nil(), "rooms"),
            SearchReindexQueueTarget::Media => (Uuid::nil(), "media"),
            SearchReindexQueueTarget::Users => (Uuid::nil(), "users"),
            SearchReindexQueueTarget::AuditLogEntries(id) => (*id, "audit_log_entries"),
        };
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

    async fn search_reindex_queue_delete(
        &mut self,
        target: SearchReindexQueueTarget,
    ) -> Result<()> {
        let (target_id, target_type) = match target {
            SearchReindexQueueTarget::Messages(id) => (*id, "messages"),
            SearchReindexQueueTarget::Channels => (Uuid::nil(), "channels"),
            SearchReindexQueueTarget::Rooms => (Uuid::nil(), "rooms"),
            SearchReindexQueueTarget::Media => (Uuid::nil(), "media"),
            SearchReindexQueueTarget::Users => (Uuid::nil(), "users"),
            SearchReindexQueueTarget::AuditLogEntries(id) => (*id, "audit_log_entries"),
        };
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

    async fn search_reindex_queue_poll(&mut self, limit: u32) -> Result<Vec<SearchReindexQueue>> {
        let mut conn = self.acquire().await?;
        let rows = query!(
            r#"SELECT target_id, target_type, last_id FROM search_reindex_queue ORDER BY updated_at ASC LIMIT $1"#,
            limit as i64
        )
        .fetch_all(conn.ext())
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let target = match r.target_type.as_str() {
                    "messages" => SearchReindexQueueTarget::Messages(r.target_id.into()),
                    "channels" => SearchReindexQueueTarget::Channels,
                    "rooms" => SearchReindexQueueTarget::Rooms,
                    "media" => SearchReindexQueueTarget::Media,
                    "users" => SearchReindexQueueTarget::Users,
                    "audit_log_entries" => SearchReindexQueueTarget::AuditLogEntries(r.target_id.into()),
                    _ => unreachable!("unknown target type: {}", r.target_type),
                };
                SearchReindexQueue {
                    target,
                    last_item_id: r.last_id,
                }
            })
            .collect())
    }

    async fn search_reindex_queue_clear(&mut self) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!("TRUNCATE search_reindex_queue")
            .execute(conn.ext())
            .await?;
        Ok(())
    }

    async fn search_reindex_queue_reset_all_audit_logs(&mut self) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"INSERT INTO search_reindex_queue (target_id, target_type) SELECT id, 'audit_log_entries' FROM room WHERE deleted_at IS NULL ON CONFLICT (target_id, target_type) DO NOTHING"#,
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn search_reindex_queue_reset_room(&mut self, room_id: RoomId) -> Result<()> {
        let mut conn = self.begin_tx().await?;
        query!(
            r#"INSERT INTO search_reindex_queue (target_id, target_type) SELECT id, 'messages' FROM channel WHERE room_id = $1 AND deleted_at IS NULL AND archived_at IS NULL ON CONFLICT (target_id, target_type) DO NOTHING"#,
            *room_id,
        )
        .execute(conn.ext())
        .await?;
        query!(
            r#"INSERT INTO search_reindex_queue (target_id, target_type) VALUES ($1, 'audit_log_entries') ON CONFLICT (target_id, target_type) DO NOTHING"#,
            *room_id,
        )
        .execute(conn.ext())
        .await?;
        conn.commit().await?;
        Ok(())
    }

    async fn search_reindex_queue_reset_all_messages(&mut self) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            r#"INSERT INTO search_reindex_queue (target_id, target_type) SELECT id, 'messages' FROM channel WHERE deleted_at IS NULL AND archived_at IS NULL ON CONFLICT (target_id, target_type) DO NOTHING"#,
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
