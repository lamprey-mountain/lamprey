use std::sync::Arc;

use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, MessageThreadUpdate,
    MessageType, Permission, Thread, ThreadId, ThreadPatch, UserId,
};
use moka::future::Cache;

use crate::error::{Error, Result};
use crate::types::{DbMessageCreate, DbThreadPrivate};
use crate::ServerStateInner;

// TODO: split caches more
// have a cache for public data, per-user data, member counts, etc
// then only invalidate (or directly update) that one part of the cache at a time
pub struct ServiceThreads {
    state: Arc<ServerStateInner>,

    cache_thread: Cache<ThreadId, Thread>,
    cache_thread_private: Cache<(ThreadId, UserId), DbThreadPrivate>,
}

impl ServiceThreads {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache_thread: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            cache_thread_private: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
        }
    }

    pub async fn get(&self, thread_id: ThreadId, user_id: Option<UserId>) -> Result<Thread> {
        let mut thread = self
            .cache_thread
            .try_get_with(thread_id, self.state.data().thread_get(thread_id))
            .await
            .map_err(|err| err.fake_clone())?;

        if let Some(user_id) = user_id {
            let private_data = self
                .cache_thread_private
                .try_get_with(
                    (thread_id, user_id),
                    self.state.data().thread_get_private(thread_id, user_id),
                )
                .await
                .map_err(|err| err.fake_clone())?;
            thread = Thread {
                is_unread: Some(private_data.is_unread),
                last_read_id: private_data.last_read_id.map(Into::into),
                mention_count: Some(0),            // TODO
                notifications: Default::default(), // TODO
                ..thread
            }
        }

        Ok(thread)
    }

    pub async fn invalidate(&self, thread_id: ThreadId) {
        self.cache_thread.invalidate(&thread_id).await;
        self.cache_thread_private
            .invalidate_entries_if(move |(t, _), _| *t == thread_id)
            .expect("failed to invalidate");
    }

    pub async fn invalidate_user(&self, thread_id: ThreadId, user_id: UserId) {
        self.cache_thread_private
            .invalidate(&(thread_id, user_id))
            .await
    }

    pub async fn update(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        patch: ThreadPatch,
        reason: Option<String>,
    ) -> Result<Thread> {
        // check update perms
        let mut perms = self
            .state
            .services()
            .perms
            .for_thread(user_id, thread_id)
            .await?;
        perms.ensure_view()?;
        let data = self.state.data();
        let thread_old = data.thread_get(thread_id).await?;
        if thread_old.creator_id == user_id {
            perms.add(Permission::ThreadEdit);
        }
        perms.ensure(Permission::ThreadEdit)?;

        // shortcut if it wont modify the thread
        if !patch.changes(&thread_old) {
            return Err(Error::NotModified);
        }

        // update and refetch
        data.thread_update(thread_id, patch.clone()).await?;
        self.invalidate(thread_id).await;
        self.invalidate_user(thread_id, user_id).await;
        let thread_new = self.get(thread_id, Some(user_id)).await?;
        if let Some(room_id) = thread_new.room_id {
            data.audit_logs_room_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id,
                session_id: None,
                reason: reason.clone(),
                ty: AuditLogEntryType::ThreadUpdate {
                    thread_id,
                    changes: Changes::new()
                        .change("name", &thread_old.name, &thread_new.name)
                        .change(
                            "description",
                            &thread_old.description,
                            &thread_new.description,
                        )
                        .change("nsfw", &thread_old.nsfw, &thread_new.nsfw)
                        .build(),
                },
            })
            .await?;
        }

        // send update message to thread
        let update_message_id = data
            .message_create(DbMessageCreate {
                thread_id,
                attachment_ids: vec![],
                author_id: user_id,
                embeds: vec![],
                message_type: MessageType::ThreadUpdate(MessageThreadUpdate {
                    patch: ThreadPatch {
                        name: patch.name,
                        description: patch.description,
                        // tags: patch.tags,
                        tags: None,
                        nsfw: patch.nsfw,
                    },
                }),
                edited_at: None,
                created_at: None,
            })
            .await?;
        let update_message = data
            .message_get(thread_id, update_message_id, user_id)
            .await?;

        self.state
            .broadcast_thread(
                thread_new.id,
                user_id,
                MessageSync::MessageCreate {
                    message: update_message,
                },
            )
            .await?;
        let msg = MessageSync::ThreadUpdate {
            thread: thread_new.clone(),
        };
        if let Some(room_id) = thread_new.room_id {
            self.state.broadcast_room(room_id, user_id, msg).await?;
        }

        Ok(thread_new)
    }
}
