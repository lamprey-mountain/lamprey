use async_trait::async_trait;
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogFilter, PaginationDirection, PaginationQuery,
    PaginationResponse, RoomId,
};
use sqlx::{query, query_as, query_scalar, Acquire};
use uuid::Uuid;

use crate::error::Result;
use crate::types::DbAuditLogEntryStatus;
use crate::{data::DataAuditLogs, gen_paginate};

use super::{Pagination, Postgres};

struct DbAuditLogEntry {
    id: Uuid,
    room_id: Uuid,
    user_id: Uuid,
    session_id: Option<Uuid>,
    reason: Option<String>,
    data: serde_json::Value,
    status: DbAuditLogEntryStatus,
    started_at: time::PrimitiveDateTime,
    ended_at: time::PrimitiveDateTime,
    ip_addr: Option<String>,
    user_agent: Option<String>,
    application_id: Option<Uuid>,
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
        let filter_statuses: Vec<DbAuditLogEntryStatus> =
            filter.status.into_iter().map(Into::into).collect();

        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbAuditLogEntry,
                r#"
                SELECT id, room_id, user_id, session_id, reason, data, status as "status: _", started_at, ended_at, ip_addr::text, user_agent, application_id FROM audit_log
                WHERE room_id = $1 AND id > $2 AND id < $3
                AND (cardinality($6::uuid[]) = 0 OR user_id = ANY($6))
                AND (cardinality($7::text[]) = 0 OR data->>'type' = ANY($7))
                AND (cardinality($8::audit_log_entry_status[]) = 0 OR status = ANY($8))
                ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
                "#,
                *room_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
                &user_ids,
                &types,
                &filter_statuses as &[DbAuditLogEntryStatus],
            ),
            query_scalar!(
                "SELECT count(*) FROM audit_log
                WHERE room_id = $1
                AND (cardinality($2::uuid[]) = 0 OR user_id = ANY($2))
                AND (cardinality($3::text[]) = 0 OR data->>'type' = ANY($3))
                AND (cardinality($4::audit_log_entry_status[]) = 0 OR status = ANY($4))
                ",
                *room_id,
                &user_ids,
                &types,
                &filter_statuses as &[DbAuditLogEntryStatus],
            ),
            |row: DbAuditLogEntry| {
                AuditLogEntry {
                    id: row.id.into(),
                    room_id: row.room_id.into(),
                    user_id: row.user_id.into(),
                    session_id: row.session_id.map(Into::into),
                    reason: row.reason,
                    ty: serde_json::from_value(row.data).unwrap(),
                    status: row.status.into(),
                    started_at: row.started_at.into(),
                    ended_at: row.ended_at.into(),
                    ip_addr: row.ip_addr,
                    user_agent: row.user_agent,
                    application_id: row.application_id.map(Into::into),
                }
            },
            |i: &AuditLogEntry| i.id.to_string()
        )
    }

    async fn audit_logs_room_append(&self, entry: AuditLogEntry) -> Result<()> {
        let status: DbAuditLogEntryStatus = entry.status.into();
        query!(
            "
            insert into audit_log (id, room_id, user_id, session_id, reason, data, status, started_at, ended_at, ip_addr, user_agent, application_id)
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::text::inet, $11, $12)
            ",
            *entry.id,
            *entry.room_id,
            *entry.user_id,
            entry.session_id.map(|id| *id),
            entry.reason,
            serde_json::to_value(&entry.ty).unwrap(),
            status as _,
            time::PrimitiveDateTime::from(entry.started_at),
            time::PrimitiveDateTime::from(entry.ended_at),
            entry.ip_addr,
            entry.user_agent,
            entry.application_id.map(|id| *id),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
