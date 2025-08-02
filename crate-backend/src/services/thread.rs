use std::sync::Arc;

use common::v1::types::util::Diff;
use common::v1::types::{
    MessageSync, MessageThreadUpdate, MessageType, Permission, Thread, ThreadId, ThreadPatch,
    ThreadPrivate, UserId,
};
use moka::future::Cache;

use crate::error::{Error, Result};
use crate::types::DbMessageCreate;
use crate::ServerStateInner;

pub struct ServiceThreads {
    state: Arc<ServerStateInner>,

    cache_thread: Cache<ThreadId, Thread>,
    cache_thread_private: Cache<(ThreadId, UserId), ThreadPrivate>,
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
            thread = thread.with_private(private_data);
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
        let thread = data.thread_get(thread_id).await?;
        if thread.creator_id == user_id {
            perms.add(Permission::ThreadEdit);
        }
        perms.ensure(Permission::ThreadEdit)?;

        // shortcut if it wont modify the thread
        if !patch.changes(&thread) {
            return Err(Error::NotModified);
        }

        // update and refetch
        data.thread_update(thread_id, patch.clone()).await?;
        self.invalidate(thread_id).await;
        self.invalidate_user(thread_id, user_id).await;
        let thread = self.get(thread_id, Some(user_id)).await?;

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
                thread.id,
                user_id,
                None,
                MessageSync::MessageCreate {
                    message: update_message,
                },
            )
            .await?;
        let msg = MessageSync::ThreadUpdate {
            thread: thread.clone(),
        };
        if let Some(room_id) = thread.room_id {
            self.state
                .broadcast_room(room_id, user_id, reason, msg)
                .await?;
        }

        Ok(thread)
    }
}
