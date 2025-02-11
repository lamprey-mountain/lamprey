use std::sync::Arc;

use serde_json::json;
use types::util::Diff;
use types::{MessageSync, MessageType, Permission, Thread, ThreadId, ThreadPatch, UserId};

use crate::error::{Error, Result};
use crate::types::MessageCreate;
use crate::ServerState;

pub struct ServiceThreads {
    state: Arc<ServerState>,
}

impl ServiceThreads {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
    }

    pub async fn update(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        patch: ThreadPatch,
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
            perms.add(Permission::ThreadManage);
        }
        perms.ensure(Permission::ThreadManage)?;

        // shortcut if it wont modify the thread
        if !patch.changes(&thread) {
            return Err(Error::NotModified);
        }

        if let Some(new_state) = &patch.state {
            if !thread.state.can_change_to(&new_state) {
                return Err(Error::BadStatic("can't change to that state"));
            }
        };

        // update and refetch
        data.thread_update(thread_id, user_id, patch.clone())
            .await?;
        let thread = data.thread_get(thread_id, Some(user_id)).await?;

        // send update message to thread
        let update_message_id = data
            .message_create(MessageCreate {
                thread_id,
                content: Some("(thread update)".to_string()),
                attachment_ids: vec![],
                author_id: user_id,
                message_type: MessageType::ThreadUpdate,
                metadata: Some(json!({
                    "name": patch.name,
                    "description": patch.description,
                })),
                reply_id: None,
                override_name: None,
            })
            .await?;
        let update_message = data.message_get(thread_id, update_message_id).await?;

        self.state.broadcast(MessageSync::UpsertMessage {
            message: update_message,
        })?;
        let msg = MessageSync::UpsertThread {
            thread: thread.clone(),
        };
        self.state
            .broadcast_room(thread.room_id, user_id, None, msg)
            .await?;

        Ok(thread)
    }
}
