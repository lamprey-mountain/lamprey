use std::sync::Arc;

use common::v1::types::util::Diff;
use common::v1::types::{
    MessageSync, MessageThreadUpdate, MessageType, Permission, Thread, ThreadId, ThreadPatch,
    UserId,
};
use moka::future::Cache;

use crate::error::{Error, Result};
use crate::types::DbMessageCreate;
use crate::ServerStateInner;

pub struct ServiceThreads {
    state: Arc<ServerStateInner>,

    // NOTE: i need to store a custom thread per user because threads might have user-specific data (unreads)
    // TODO: don't do this
    cache_thread: Cache<(ThreadId, Option<UserId>), Thread>,
}

impl ServiceThreads {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache_thread: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
        }
    }

    pub async fn get(&self, thread_id: ThreadId, user_id: Option<UserId>) -> Result<Thread> {
        self.cache_thread
            .try_get_with(
                (thread_id, user_id),
                self.state.data().thread_get(thread_id, user_id),
            )
            .await
            .map_err(|err| err.fake_clone())
    }

    pub fn invalidate(&self, thread_id: ThreadId) {
        self.cache_thread
            .invalidate_entries_if(move |(t, _), _| *t == thread_id)
            .expect("failed to invalidate");
    }

    pub async fn invalidate_user(&self, thread_id: ThreadId, user_id: UserId) {
        self.cache_thread
            .invalidate(&(thread_id, Some(user_id)))
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
        let thread = data.thread_get(thread_id, Some(user_id)).await?;
        if thread.creator_id == user_id {
            perms.add(Permission::ThreadEdit);
        }
        perms.ensure(Permission::ThreadEdit)?;

        // shortcut if it wont modify the thread
        if !patch.changes(&thread) {
            return Err(Error::NotModified);
        }

        // update and refetch
        data.thread_update(thread_id, user_id, patch.clone())
            .await?;
        self.invalidate(thread_id);
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
                    },
                }),
            })
            .await?;
        let update_message = data.message_get(thread_id, update_message_id).await?;

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
        self.state
            .broadcast_room(thread.room_id, user_id, reason, msg)
            .await?;

        Ok(thread)
    }
}
