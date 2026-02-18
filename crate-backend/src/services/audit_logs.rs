use std::sync::Arc;

use common::v1::types::{
    audit_logs::resolve::AuditLogResolve, AuditLogEntry, AuditLogEntryId, AuditLogFilter,
    AuditLogPaginationResponse, Channel, PaginationQuery, RoomId, RoomMember, User, Webhook,
};

use crate::{error::Result, ServerStateInner};

pub struct ServiceAuditLogs {
    state: Arc<ServerStateInner>,
}

impl ServiceAuditLogs {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    pub async fn list(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<AuditLogEntryId>,
        filter: AuditLogFilter,
    ) -> Result<AuditLogPaginationResponse> {
        let data = self.state.data();

        let entries = data
            .audit_logs_room_fetch(room_id, paginate, filter.clone())
            .await?;

        let mut resolve = AuditLogResolve::default();

        for entry in &entries.items {
            resolve.add(entry);
        }

        // TODO: use get_many
        let mut threads = Vec::new();
        for thread_id in &resolve.threads {
            if let Ok(thread) = data.channel_get(*thread_id).await {
                threads.push(thread);
            }
        }

        let mut users = Vec::new();
        for user_id in &resolve.users {
            if let Ok(user) = data.user_get(*user_id).await {
                users.push(user);
            }
        }

        let mut room_members = Vec::new();
        if let Ok(room_id) = room_id.try_into() {
            for user_id in &resolve.users {
                if let Ok(member) = data.room_member_get(room_id, *user_id).await {
                    room_members.push(member);
                }
            }
        }

        let mut webhooks = Vec::new();
        for webhook_id in &resolve.webhooks {
            if let Ok(webhook) = data.webhook_get(*webhook_id).await {
                webhooks.push(webhook);
            }
        }

        Ok(AuditLogPaginationResponse {
            audit_log_entries: entries.items,
            threads,
            users,
            room_members,
            webhooks,
            has_more: entries.has_more,
            cursor: entries.cursor,
        })
    }
}
