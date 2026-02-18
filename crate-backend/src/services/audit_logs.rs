use std::sync::Arc;

use common::v1::types::{
    audit_logs::resolve::AuditLogResolve, AuditLogEntryId, AuditLogFilter,
    AuditLogPaginationResponse, PaginationQuery, RoomId,
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

        let srv = self.state.services();
        let cached_room = srv.cache.load_room(room_id).await?;

        let mut threads = Vec::new();
        let mut missing_threads = Vec::new();

        for thread_id in &resolve.threads {
            if let Some(chan) = cached_room.channels.get(thread_id) {
                threads.push(chan.clone());
            } else if let Some(thread) = cached_room.threads.get(thread_id) {
                threads.push(thread.thread.read().await.clone());
            } else {
                missing_threads.push(*thread_id);
            }
        }

        if !missing_threads.is_empty() {
            let mut more_threads = srv.channels.get_many(&missing_threads, None).await?;
            threads.append(&mut more_threads);
        }

        // NOTE: will this always remove everything?
        threads.retain(|chan| chan.archived_at.is_none() || chan.ty.is_thread());

        let user_ids: Vec<_> = resolve.users.iter().cloned().collect();
        let users = srv.users.get_many(&user_ids).await?;

        let mut room_members = Vec::new();
        for user_id in &resolve.users {
            if let Some(member) = cached_room.members.get(user_id) {
                room_members.push(member.member.clone());
            }
        }

        let mut webhooks = Vec::new();
        for webhook_id in &resolve.webhooks {
            if let Ok(webhook) = data.webhook_get(*webhook_id).await {
                webhooks.push(webhook);
            }
        }

        // TODO: batch fetch, include channel_id in query to use index
        let mut tags = Vec::new();
        for tag_id in &resolve.tags {
            if let Ok(tag) = data.tag_get(*tag_id).await {
                tags.push(tag);
            }
        }

        Ok(AuditLogPaginationResponse {
            audit_log_entries: entries.items,
            threads,
            users,
            room_members,
            webhooks,
            tags,
            has_more: entries.has_more,
            cursor: entries.cursor,
        })
    }
}
