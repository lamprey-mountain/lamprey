use async_trait::async_trait;
use common::v1::types::{
    AuditLog, AuditLogId, MessageSync, PaginationDirection, PaginationQuery, PaginationResponse,
    RoomId, UserId,
};
use sqlx::{query, query_as, query_scalar, Acquire};
use uuid::Uuid;

use crate::error::Result;
use crate::{data::DataAuditLogs, gen_paginate};

use super::{Pagination, Postgres};

struct DbAuditLog {
    id: Uuid,
    user_id: Uuid,
    reason: Option<String>,
    payload: serde_json::Value,
    payload_prev: Option<serde_json::Value>,
}

#[async_trait]
impl DataAuditLogs for Postgres {
    async fn audit_logs_room_fetch(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<AuditLogId>,
    ) -> Result<PaginationResponse<AuditLog>> {
        let p: Pagination<_> = paginate.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbAuditLog,
                "
            	SELECT id, user_id, reason, payload, payload_prev FROM audit_log
            	WHERE room_id = $1 AND id > $2 AND id < $3
            	ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
                ",
                room_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM audit_log WHERE room_id = $1",
                room_id.into_inner()
            ),
            |row| AuditLog {
                id: row.id.into(),
                room_id,
                user_id: row.user_id.into(),
                reason: row.reason,
                payload: serde_json::from_value(row.payload).expect("corrupted data in db!"),
                payload_prev: row
                    .payload_prev
                    .map(|p| serde_json::from_value(p).expect("corrupted data in db!")),
            },
            |i| i.id.to_string()
        )
    }

    async fn audit_logs_room_append(
        &self,
        room_id: RoomId,
        user_id: UserId,
        reason: Option<String>,
        payload: MessageSync,
    ) -> Result<()> {
        let id = Uuid::now_v7();
        let target_id = payload.get_audit_target_id().expect("couldn't get id?");
        let payload = serde_json::to_value(payload)?;
        // NOTE: message shouldn't have prev (works for now, but might have issues later)
        query!(
            "
            insert into audit_log (id, room_id, user_id, reason, payload, payload_prev)
        	values ($1, $2, $3, $4, $5, (
                select payload from audit_log
                where payload->'thread'->>'id' = $6
                or payload->'user'->>'id' = $6
                or payload->'role'->>'id' = $6
                or payload->'member'->>'user_id' = $6
                order by id desc limit 1
        	))
        	",
            id,
            room_id.into_inner(),
            user_id.into_inner(),
            reason,
            payload,
            target_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
