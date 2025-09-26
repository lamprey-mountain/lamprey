use std::sync::Arc;
use std::time::Duration;

use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, MessageThreadRename,
    MessageType, Permission, Thread, ThreadId, ThreadPatch, ThreadType, User, UserId,
};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use moka::future::Cache;
use time::OffsetDateTime;

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
    cache_thread_recipients: Cache<ThreadId, Vec<User>>,
    typing: Cache<(ThreadId, UserId), OffsetDateTime>,
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
            cache_thread_recipients: Cache::builder()
                .max_capacity(10_000)
                .support_invalidation_closures()
                .build(),
            typing: Cache::builder()
                .max_capacity(100_000)
                .time_to_live(Duration::from_secs(10))
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

            let recipients = self
                .cache_thread_recipients
                .try_get_with(thread_id, async {
                    if !matches!(thread.ty, ThreadType::Dm | ThreadType::Gdm) {
                        return Ok(vec![]);
                    }

                    let members = self
                        .state
                        .data()
                        .thread_member_list(
                            thread_id,
                            common::v1::types::PaginationQuery {
                                from: None,
                                to: None,
                                dir: None,
                                limit: Some(1024),
                            },
                        )
                        .await?;
                    let data = self.state.data();
                    let mut futures = FuturesOrdered::new();
                    for member in members.items {
                        futures.push_back(data.user_get(member.user_id));
                    }
                    let mut users = vec![];
                    while let Some(user) = futures.next().await {
                        users.push(user?);
                    }
                    Result::Ok(users)
                })
                .await
                .map_err(|err| err.fake_clone())?;
            let recipients: Vec<_> = recipients.into_iter().filter(|u| u.id != user_id).collect();

            thread = Thread {
                recipient: recipients.first().cloned(),
                recipients,
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
        let srv = self.state.services();
        let thread_old = srv.threads.get(thread_id, None).await?;
        if thread_old.archived_at.is_some() {
            return Err(Error::BadStatic("thread is archived"));
        }
        if thread_old.deleted_at.is_some() {
            return Err(Error::BadStatic("thread is removed"));
        }
        if thread_old.locked {
            perms.ensure(Permission::ThreadLock)?;
        }
        if thread_old.creator_id == user_id {
            perms.add(Permission::ThreadEdit);
        }
        perms.ensure(Permission::ThreadEdit)?;

        // shortcut if it wont modify the thread
        if !patch.changes(&thread_old) {
            return Err(Error::NotModified);
        }
        if patch.bitrate.is_some_and(|b| b.is_some_and(|b| b > 393216)) {
            return Err(Error::BadStatic("bitrate is too high"));
        }
        if thread_old.ty != ThreadType::Voice && patch.bitrate.is_some() {
            return Err(Error::BadStatic("cannot set bitrate for non voice thread"));
        }
        if thread_old.ty != ThreadType::Voice && patch.user_limit.is_some() {
            return Err(Error::BadStatic(
                "cannot set user_limit for non voice thread",
            ));
        }

        // update and refetch
        data.thread_update(thread_id, patch.clone()).await?;
        self.invalidate(thread_id).await;
        self.invalidate_user(thread_id, user_id).await;
        let thread_new = self.get(thread_id, Some(user_id)).await?;
        if let Some(room_id) = thread_new.room_id {
            self.state
                .audit_log_append(AuditLogEntry {
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
                            .change("bitrate", &thread_old.bitrate, &thread_new.bitrate)
                            .change("user_limit", &thread_old.user_limit, &thread_new.user_limit)
                            .build(),
                    },
                })
                .await?;
        }

        if thread_old.name != thread_new.name {
            // send thread renamed message to thread
            let rename_message_id = data
                .message_create(DbMessageCreate {
                    thread_id,
                    attachment_ids: vec![],
                    author_id: user_id,
                    embeds: vec![],
                    message_type: MessageType::ThreadRename(MessageThreadRename {
                        name_new: thread_new.name.clone(),
                        name_old: thread_old.name,
                    }),
                    edited_at: None,
                    created_at: None,
                })
                .await?;
            let rename_message = data
                .message_get(thread_id, rename_message_id, user_id)
                .await?;
            self.state
                .broadcast_thread(
                    thread_id,
                    user_id,
                    MessageSync::MessageCreate {
                        message: rename_message,
                    },
                )
                .await?;
        }

        let msg = MessageSync::ThreadUpdate {
            thread: thread_new.clone(),
        };
        if let Some(room_id) = thread_new.room_id {
            self.state.broadcast_room(room_id, user_id, msg).await?;
        }

        Ok(thread_new)
    }

    pub async fn typing_set(&self, thread_id: ThreadId, user_id: UserId, until: OffsetDateTime) {
        self.typing.insert((thread_id, user_id), until).await;
    }

    pub fn typing_list(&self) -> Vec<(ThreadId, UserId, OffsetDateTime)> {
        self.typing
            .iter()
            .map(|(key, until)| (key.0, key.1, until))
            .collect()
    }
}
