use async_trait::async_trait;
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogFilter, PaginationDirection, PaginationQuery,
    PaginationResponse, RoomId,
};
use sqlx::{query, query_as, query_scalar, Acquire};
use uuid::Uuid;

use crate::error::Result;
use crate::{data::DataAuditLogs, gen_paginate};

use super::{Pagination, Postgres};

struct DbAuditLogEntry {
    id: Uuid,
    room_id: Uuid,
    user_id: Uuid,
    session_id: Option<Uuid>,
    reason: Option<String>,
    data: serde_json::Value,
}

#[async_trait]
impl DataAuditLogs for Postgres {
    async fn audit_logs_room_fetch(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<AuditLogEntryId>,
        filter: AuditLogFilter,
    ) -> Result<PaginationResponse<AuditLogEntry>> {
        let p: Pagination<_> = paginate.try_into()?;

        let user_ids: Vec<Uuid> = filter.user_id.into_iter().map(|id| *id).collect();
        let types = filter.ty;

        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbAuditLogEntry,
                "
                SELECT id, room_id, user_id, session_id, reason, data FROM audit_log
                WHERE room_id = $1 AND id > $2 AND id < $3
                AND (cardinality($6::uuid[]) = 0 OR user_id = ANY($6))
                AND (cardinality($7::text[]) = 0 OR data->>'type' = ANY($7))
                ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
                ",
                *room_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
                &user_ids,
                &types,
            ),
            query_scalar!(
                "SELECT count(*) FROM audit_log
                WHERE room_id = $1
                AND (cardinality($2::uuid[]) = 0 OR user_id = ANY($2))
                AND (cardinality($3::text[]) = 0 OR data->>'type' = ANY($3))
                ",
                *room_id,
                &user_ids,
                &types,
            ),
            |row: DbAuditLogEntry| {
                AuditLogEntry {
                    id: row.id.into(),
                    room_id: row.room_id.into(),
                    user_id: row.user_id.into(),
                    session_id: row.session_id.map(Into::into),
                    reason: row.reason,
                    ty: serde_json::from_value(row.data).unwrap(),
                }
            },
            |i: &AuditLogEntry| i.id.to_string()
        )
    }

    async fn audit_logs_room_append(&self, entry: AuditLogEntry) -> Result<()> {
        query!(
            "
            insert into audit_log (id, room_id, user_id, session_id, reason, data)
            values ($1, $2, $3, $4, $5, $6)
            ",
            *entry.id,
            *entry.room_id,
            *entry.user_id,
            entry.session_id.map(|id| *id),
            entry.reason,
            serde_json::to_value(&entry.ty).unwrap(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
