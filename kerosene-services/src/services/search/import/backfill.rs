use std::{sync::Arc, time::Duration};

use common::v1::types::{
    AuditLogFilter, ChannelId, MediaVerId, PaginationDirection, PaginationQuery, RoomId, UserId,
};
use dashmap::DashSet;
use lamprey_backend_core::types::data::{SearchReindexQueue, SearchReindexQueueTarget};
use tantivy::Term;
use tokio::task::JoinSet;
use tracing::error;
use uuid::Uuid;

use crate::{
    ServerStateInner,
    services::search::{index::AsyncIndexHandle, util::SCHEMA},
};

#[derive(Clone)]
pub struct BackfillEtl {
    inner: Arc<BackfillEtlInner>,
}

pub struct BackfillEtlInner {
    s: Arc<ServerStateInner>,
    index: AsyncIndexHandle,
    active: DashSet<SearchReindexQueueTarget>,
}

impl BackfillEtl {
    pub fn new(s: Arc<ServerStateInner>, index: AsyncIndexHandle) -> Self {
        BackfillEtl {
            inner: Arc::new(BackfillEtlInner {
                s,
                index,
                active: DashSet::new(),
            }),
        }
    }

    pub async fn spawn(self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        let mut workers = JoinSet::new();

        loop {
            interval.tick().await;

            // clean up completed workers
            while let Some(res) = workers.try_join_next() {
                if let Err(e) = res {
                    error!("Backfill worker panicked: {e}");
                }
            }

            // (re)start workers
            let concurrency_limit = self.inner.s.config.search.import_concurrency as i32;
            let available_slots = concurrency_limit.saturating_sub(workers.len() as i32) as u32;
            if available_slots == 0 {
                continue;
            }

            let mut data = self.inner.s.data();
            let queues = match data.search_reindex_queue_poll(available_slots).await {
                Ok(q) => q,
                Err(e) => {
                    error!("Failed to list reindex queue: {e}");
                    continue;
                }
            };

            for queue in queues {
                workers.spawn(Arc::clone(&self.inner).spawn_worker(queue));
            }
        }
    }
}

impl BackfillEtlInner {
    async fn spawn_worker(self: Arc<Self>, queue: SearchReindexQueue) {
        if !self.active.insert(queue.target.clone()) {
            return;
        }

        match &queue.target {
            SearchReindexQueueTarget::Messages(id) => self.spawn_messages(*id).await,
            SearchReindexQueueTarget::Channels => self.spawn_channels().await,
            SearchReindexQueueTarget::Rooms => self.spawn_rooms().await,
            SearchReindexQueueTarget::Users => self.spawn_users().await,
            SearchReindexQueueTarget::Media => self.spawn_media().await,
            SearchReindexQueueTarget::AuditLogEntries(id) => self.spawn_audit_logs(*id).await,
        }

        let mut data = self.s.data();
        if let Err(e) = data.search_reindex_queue_delete(queue.target.clone()).await {
            error!("Failed to delete reindex queue entry: {e}");
        }

        self.active.remove(&queue.target);
    }

    async fn spawn_audit_logs(&self, room_id: RoomId) {
        let mut data = self.s.data();
        let mut last_id: Option<Uuid> = None;

        loop {
            let entries = match data
                .audit_logs_room_fetch(
                    room_id,
                    PaginationQuery {
                        from: last_id.map(|id| id.into()),
                        to: None,
                        dir: Some(PaginationDirection::B),
                        limit: Some(100),
                    },
                    AuditLogFilter::default(),
                )
                .await
            {
                Ok(e) => e,
                Err(err) => {
                    error!("failed to fetch audit log entries: {err}");
                    break;
                }
            };

            if entries.items.is_empty() {
                break;
            }

            let mut batch = Vec::with_capacity(entries.items.len());
            for entry in &entries.items {
                match SCHEMA.transform_audit_log_entry(entry) {
                    Ok(doc) => {
                        let term = Term::from_field_text(SCHEMA.id, &entry.id.to_string());
                        batch.push((term, doc));
                    }
                    Err(e) => error!("failed to transform audit log entry {}: {e}", entry.id),
                }
            }

            if !batch.is_empty() {
                if let Err(e) = self.index.update_documents(batch).await {
                    error!("failed to update index: {e}");
                }
                let _ = self.index.lazy_commit().await;
            }

            if let Some(last) = entries.items.last() {
                last_id = Some(*last.id);
            }

            if !entries.has_more {
                break;
            }

            // avoid blocking the executor for too long
            tokio::task::yield_now().await;
        }

        let _ = self.index.commit().await;
    }

    async fn spawn_users(&self) {
        let mut data = self.s.data();
        let mut last_id: Option<UserId> = None;

        loop {
            let res = match data
                .user_list(
                    PaginationQuery {
                        from: last_id,
                        to: None,
                        dir: None,
                        limit: Some(100),
                    },
                    None,
                )
                .await
            {
                Ok(u) => u,
                Err(err) => {
                    error!("failed to fetch users: {err}");
                    break;
                }
            };

            if res.items.is_empty() {
                break;
            }

            let mut batch = Vec::with_capacity(res.items.len());
            for user in &res.items {
                // TODO: log error
                if let Ok(doc) = SCHEMA.transform_user(user) {
                    let term = Term::from_field_text(SCHEMA.id, &user.id.to_string());
                    batch.push((term, doc));
                }
            }

            if !batch.is_empty() {
                if let Err(e) = self.index.update_documents(batch).await {
                    error!("failed to update index: {e}");
                }
                let _ = self.index.lazy_commit().await;
            }

            if let Some(last) = res.items.last() {
                last_id = Some(last.id);
            }

            if !res.has_more {
                break;
            }

            // avoid blocking the executor for too long
            tokio::task::yield_now().await;
        }

        let _ = self.index.commit().await;
    }

    async fn spawn_media(&self) {
        let mut data = self.s.data();
        let mut last_version_id: Option<MediaVerId> = None;

        loop {
            let media_list = match data.media_list_indexed(last_version_id, 100).await {
                Ok(m) => m,
                Err(err) => {
                    error!("failed to fetch media: {err}");
                    break;
                }
            };

            if media_list.is_empty() {
                break;
            }

            let mut batch = Vec::with_capacity(media_list.len());
            for media in &media_list {
                match SCHEMA.transform_media(media) {
                    Ok(doc) => {
                        let term = Term::from_field_text(SCHEMA.id, &media.id.to_string());
                        batch.push((term, doc));
                    }
                    Err(e) => error!("failed to transform media {}: {e}", media.id),
                }
            }

            if !batch.is_empty() {
                if let Err(e) = self.index.update_documents(batch).await {
                    error!("failed to update index: {e}");
                }
                let _ = self.index.lazy_commit().await;
            }

            if let Some(last) = media_list.last() {
                last_version_id = Some(last.version_id);
            }

            if media_list.len() < 100 {
                break;
            }

            // avoid blocking the executor for too long
            tokio::task::yield_now().await;
        }

        let _ = self.index.commit().await;
    }

    async fn spawn_messages(&self, channel_id: ChannelId) {
        let srv = self.s.services();
        let chan = match srv.channels.get(channel_id, None).await {
            Ok(chan) => chan,
            Err(err) => {
                error!("failed to get channel: {err}");
                return;
            }
        };

        let mut last_id: Option<Uuid> = None;

        loop {
            let messages = match srv
                .messages
                .list(
                    channel_id,
                    None,
                    PaginationQuery {
                        from: last_id.map(|id| id.into()),
                        to: None,
                        dir: Some(PaginationDirection::B),
                        limit: Some(100),
                    },
                )
                .await
            {
                Ok(m) => m,
                Err(err) => {
                    error!("failed to fetch messages: {err}");
                    break;
                }
            };

            if messages.items.is_empty() {
                break;
            }

            let mut batch = Vec::with_capacity(messages.items.len());
            for message in &messages.items {
                let term = Term::from_field_text(SCHEMA.id, &message.id.to_string());
                let doc = SCHEMA
                    .transform_message(message, chan.room_id, chan.parent_id)
                    .unwrap();
                batch.push((term, doc));
            }

            if let Err(e) = self.index.update_documents(batch).await {
                error!("failed to update index: {e}");
            }
            let _ = self.index.lazy_commit().await;

            if let Some(last) = messages.items.last() {
                last_id = Some(*last.id);
            }

            if !messages.has_more {
                break;
            }

            // avoid blocking the executor for too long
            tokio::task::yield_now().await;
        }

        let _ = self.index.commit().await;
    }

    async fn spawn_channels(&self) {
        let mut data = self.s.data();
        let mut last_id: Option<ChannelId> = None;

        loop {
            let res = match data
                .channel_list_all(PaginationQuery {
                    from: last_id,
                    to: None,
                    dir: None,
                    limit: Some(100),
                })
                .await
            {
                Ok(r) => r,
                Err(err) => {
                    error!("failed to fetch channels: {err}");
                    break;
                }
            };

            if res.items.is_empty() {
                break;
            }

            let mut batch = Vec::with_capacity(res.items.len());
            for channel in &res.items {
                let srv = self.s.services();
                let first_message = srv.messages.get_first(channel.id, None).await.ok();
                match SCHEMA.transform_channel(channel, first_message.as_ref()) {
                    Ok(doc) => {
                        let term = Term::from_field_text(SCHEMA.id, &channel.id.to_string());
                        batch.push((term, doc));
                    }
                    Err(e) => error!("failed to transform channel {}: {e}", channel.id),
                }
            }

            if !batch.is_empty() {
                if let Err(e) = self.index.update_documents(batch).await {
                    error!("failed to update index: {e}");
                }
                let _ = self.index.lazy_commit().await;
            }

            if let Some(last) = res.items.last() {
                last_id = Some(last.id);
            }

            if !res.has_more {
                break;
            }

            tokio::task::yield_now().await;
        }

        let _ = self.index.commit().await;
    }

    async fn spawn_rooms(&self) {
        let mut data = self.s.data();
        let mut last_id: Option<RoomId> = None;

        loop {
            let res = match data
                .room_list_all(PaginationQuery {
                    from: last_id,
                    to: None,
                    dir: None,
                    limit: Some(100),
                })
                .await
            {
                Ok(r) => r,
                Err(err) => {
                    error!("failed to fetch rooms: {err}");
                    break;
                }
            };

            if res.items.is_empty() {
                break;
            }

            let mut batch = Vec::with_capacity(res.items.len());
            for room in &res.items {
                if let Ok(doc) = SCHEMA.transform_room(room) {
                    let term = Term::from_field_text(SCHEMA.id, &room.id.to_string());
                    batch.push((term, doc));
                }
            }

            if !batch.is_empty() {
                if let Err(e) = self.index.update_documents(batch).await {
                    error!("failed to update index: {e}");
                }
                let _ = self.index.lazy_commit().await;
            }

            if let Some(last) = res.items.last() {
                last_id = Some(last.id);
            }

            if !res.has_more {
                break;
            }

            tokio::task::yield_now().await;
        }

        let _ = self.index.commit().await;
    }
}
