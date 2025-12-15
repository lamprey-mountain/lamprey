use async_trait::async_trait;
use sqlx::{query, query_file, query_scalar};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

use crate::{
    data::DataAdmin, error::Result, services::admin::AdminCollectGarbageMode, types::MediaId, Error,
};

use super::Postgres;

#[async_trait]
impl DataAdmin for Postgres {
    async fn gc_room_analytics(&self, mode: AdminCollectGarbageMode) -> Result<u64> {
        match mode {
            AdminCollectGarbageMode::Sweep => {
                let interval = Duration::days(crate::consts::RETENTION_ROOM_ANALYTICS as i64);
                let cutoff = OffsetDateTime::now_utc() - interval;
                let cutoff_primitive = PrimitiveDateTime::new(cutoff.date(), cutoff.time());

                let r1 = query!("DELETE FROM metric_room WHERE ts < $1", cutoff_primitive)
                    .execute(&self.pool)
                    .await?;

                let r2 = query!("DELETE FROM metric_channel WHERE ts < $1", cutoff_primitive)
                    .execute(&self.pool)
                    .await?;

                Ok(r1.rows_affected() + r2.rows_affected())
            }
            _ => Err(Error::Unimplemented),
        }
    }

    async fn gc_messages(&self, mode: AdminCollectGarbageMode) -> Result<u64> {
        match mode {
            AdminCollectGarbageMode::Sweep => {
                let result = sqlx::raw_sql(include_str!("../../../sql/purge_messages.sql"))
                    .execute(&self.pool)
                    .await?;
                Ok(result.rows_affected())
            }
            _ => Err(Error::Unimplemented),
        }
    }

    async fn gc_media_mark(&self) -> Result<u64> {
        // TODO: return sum(media_size) somehow
        let result = query_file!("sql/gc_media.sql").execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    async fn gc_media_get_sweep_candidates(&self, limit: u32) -> Result<Vec<MediaId>> {
        let rows = query!(
            "select id from media where deleted_at is not null limit $1",
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.id.into()).collect())
    }

    async fn gc_media_delete_swept(&self, ids: &[MediaId]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let ids: Vec<Uuid> = ids.iter().map(|id| id.into_inner()).collect();
        let result = query!("delete from media where id = ANY($1)", &ids)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn gc_media_count_deleted(&self) -> Result<u64> {
        let count = query_scalar!("SELECT COUNT(*) FROM media WHERE deleted_at IS NOT NULL")
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0);
        Ok(count as u64)
    }

    async fn gc_sessions(&self, mode: AdminCollectGarbageMode) -> Result<u64> {
        match mode {
            AdminCollectGarbageMode::Sweep => {
                let result = sqlx::raw_sql(include_str!("../../../sql/purge_sessions.sql"))
                    .execute(&self.pool)
                    .await?;
                Ok(result.rows_affected())
            }
            _ => Err(Error::Unimplemented),
        }
    }

    async fn gc_audit_logs(&self, mode: AdminCollectGarbageMode) -> Result<u64> {
        match mode {
            AdminCollectGarbageMode::Sweep => {
                let interval = Duration::days(crate::consts::RETENTION_AUDIT_LOG as i64);
                let cutoff = OffsetDateTime::now_utc() - interval;
                let cutoff_primitive = PrimitiveDateTime::new(cutoff.date(), cutoff.time());
                // TODO: extract and index on created_at
                let result = query!(
                    "DELETE FROM audit_log WHERE extract_timestamp_from_uuid_v7(id) < $1",
                    cutoff_primitive
                )
                .execute(&self.pool)
                .await?;

                Ok(result.rows_affected())
            }
            _ => Err(Error::Unimplemented),
        }
    }
}
