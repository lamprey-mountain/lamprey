use async_trait::async_trait;
use sqlx::query;

use crate::{data::DataAdmin, error::Result, services::admin::AdminCollectGarbageMode};

use super::Postgres;

#[async_trait]
impl DataAdmin for Postgres {
    async fn gc_room_analytics(&self, mode: AdminCollectGarbageMode) -> Result<u64> {
        match mode {
            AdminCollectGarbageMode::Sweep => {
                let rows =
                    query!("DELETE FROM room_analytics WHERE created_at < NOW() - INTERVAL '180 days'")
                        .execute(&self.pool)
                        .await?;
                Ok(rows.rows_affected())
            }
            _ => todo!(),
        }
    }

    async fn gc_messages(&self, mode: AdminCollectGarbageMode) -> Result<u64> {
        match mode {
            AdminCollectGarbageMode::Sweep => {
                let result = sqlx::raw_sql(include_str!("../../sql/purge_messages.sql"))
                    .execute(&self.pool)
                    .await?;
                Ok(result.rows_affected())
            }
            _ => todo!(),
        }
    }

    async fn gc_media(&self, _mode: AdminCollectGarbageMode) -> Result<u64> {
        todo!("media gc involves S3 and is more complex. this should be handled in the service layer.")
    }

    async fn gc_sessions(&self, mode: AdminCollectGarbageMode) -> Result<u64> {
        match mode {
            AdminCollectGarbageMode::Sweep => {
                let result = sqlx::raw_sql(include_str!("../../sql/purge_sessions.sql"))
                    .execute(&self.pool)
                    .await?;
                Ok(result.rows_affected())
            }
            _ => todo!(),
        }
    }

    async fn gc_audit_logs(&self, mode: AdminCollectGarbageMode) -> Result<u64> {
        match mode {
            AdminCollectGarbageMode::Sweep => {
                let rows = query!("DELETE FROM audit_log WHERE created_at < NOW() - INTERVAL '90 days'")
                    .execute(&self.pool)
                    .await?;
                Ok(rows.rows_affected())
            }
            _ => todo!(),
        }
    }
}