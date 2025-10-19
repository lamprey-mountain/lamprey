use std::sync::Arc;
use std::time::Duration;

use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Channel, ChannelId, ChannelPatch,
    ChannelType, MessageSync, MessageThreadRename, MessageType, Permission, User, UserId,
};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use moka::future::Cache;
use time::OffsetDateTime;

use crate::error::{Error, Result};
use crate::types::{DbChannelPrivate, DbMessageCreate};
use crate::ServerStateInner;

// TODO: split caches more
// have a cache for public data, per-user data, member counts, etc
// then only invalidate (or directly update) that one part of the cache at a time
pub struct ServiceThreads {
    state: Arc<ServerStateInner>,
    cache_thread: Cache<ChannelId, Channel>,
    cache_thread_private: Cache<(ChannelId, UserId), DbChannelPrivate>,
    cache_thread_recipients: Cache<ChannelId, Vec<User>>,
    typing: Cache<(ChannelId, UserId), OffsetDateTime>,
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

    pub async fn get(&self, channel_id: ChannelId, user_id: Option<UserId>) -> Result<Channel> {
        let mut thread = self
            .cache_thread
            .try_get_with(channel_id, self.state.data().channel_get(channel_id))
            .await
            .map_err(|err| err.fake_clone())?;

        if let Some(user_id) = user_id {
            let private_data = self
                .cache_thread_private
                .try_get_with(
                    (channel_id, user_id),
                    self.state.data().channel_get_private(channel_id, user_id),
                )
                .await
                .map_err(|err| err.fake_clone())?;

            let state = self.state.clone();
            let thread_ty = thread.ty;
            let recipients = self
                .cache_thread_recipients
                .try_get_with(channel_id, async move {
                    if !matches!(thread_ty, ChannelType::Dm | ChannelType::Gdm) {
                        return Ok(vec![]);
                    }

                    let members = state
                        .data()
                        .thread_member_list(
                            channel_id,
                            common::v1::types::PaginationQuery {
                                from: None,
                                to: None,
                                dir: None,
                                limit: Some(1024),
                            },
                        )
                        .await?;
                    let srv = state.services();
                    let mut futures = FuturesOrdered::new();
                    for member in members.items {
                        futures.push_back(srv.users.get(member.user_id, Some(user_id)));
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

            let user_config = self
                .state
                .data()
                .user_config_channel_get(user_id, channel_id)
                .await?;

            thread = Channel {
                recipients,
                is_unread: Some(private_data.is_unread),
                last_read_id: private_data.last_read_id.map(Into::into),
                mention_count: Some(0), // TODO
                user_config: Some(user_config),
                ..thread
            }
        }

        let members = self.state.data().thread_member_list_all(channel_id).await?;

        let mut online_count = 0;
        for member in members {
            if self
                .state
                .services()
                .users
                .status_get(member.user_id)
                .status
                .is_online()
            {
                online_count += 1;
            }
        }
        thread.online_count = online_count;

        Ok(thread)
    }

    pub async fn invalidate(&self, thread_id: ChannelId) {
        self.cache_thread.invalidate(&thread_id).await;
        self.cache_thread_private
            .invalidate_entries_if(move |(t, _), _| *t == thread_id)
            .expect("failed to invalidate");
    }

    pub async fn invalidate_user(&self, thread_id: ChannelId, user_id: UserId) {
        self.cache_thread_private
            .invalidate(&(thread_id, user_id))
            .await
    }

    pub async fn update(
        &self,
        user_id: UserId,
        thread_id: ChannelId,
        patch: ChannelPatch,
        reason: Option<String>,
    ) -> Result<Channel> {
        // check update perms
        let mut perms = self
            .state
            .services()
            .perms
            .for_channel(user_id, thread_id)
            .await?;
        perms.ensure(Permission::ViewChannel)?;
        let data = self.state.data();
        let srv = self.state.services();
        let chan_old = srv.channels.get(thread_id, None).await?;
        if chan_old.archived_at.is_some() {
            return Err(Error::BadStatic("thread is archived"));
        }
        if chan_old.deleted_at.is_some() {
            return Err(Error::BadStatic("thread is removed"));
        }
        if chan_old.locked {
            perms.ensure(Permission::ThreadLock)?;
        }
        if chan_old.creator_id == user_id {
            perms.add(Permission::ThreadEdit);
        }
        perms.ensure(Permission::ThreadEdit)?;

        // shortcut if it wont modify the thread
        if !patch.changes(&chan_old) {
            return Ok(chan_old);
        }
        if patch.bitrate.is_some_and(|b| b.is_some_and(|b| b > 393216)) {
            return Err(Error::BadStatic("bitrate is too high"));
        }
        if chan_old.ty != ChannelType::Voice && patch.bitrate.is_some() {
            return Err(Error::BadStatic("cannot set bitrate for non voice thread"));
        }
        if chan_old.ty != ChannelType::Voice && patch.user_limit.is_some() {
            return Err(Error::BadStatic(
                "cannot set user_limit for non voice thread",
            ));
        }

        if let Some(Some(icon)) = patch.icon {
            if chan_old.ty != ChannelType::Gdm {
                return Err(Error::BadStatic("only gdm threads can have icons"));
            }
            let (media, _) = data.media_select(icon).await?;
            if !matches!(
                media.source.info,
                common::v1::types::MediaTrackInfo::Image(_)
            ) {
                return Err(Error::BadStatic("media not an image"));
            }
        }

        // update and refetch
        data.channel_update(thread_id, patch.clone()).await?;
        self.invalidate(thread_id).await;
        self.invalidate_user(thread_id, user_id).await;
        let chan_new = self.get(thread_id, Some(user_id)).await?;
        if let Some(room_id) = chan_new.room_id {
            self.state
                .audit_log_append(AuditLogEntry {
                    id: AuditLogEntryId::new(),
                    room_id,
                    user_id,
                    session_id: None,
                    reason: reason.clone(),
                    ty: AuditLogEntryType::ChannelUpdate {
                        channel_id: thread_id,
                        channel_type: chan_new.ty,
                        changes: Changes::new()
                            .change("name", &chan_old.name, &chan_new.name)
                            .change("description", &chan_old.description, &chan_new.description)
                            .change("icon", &chan_old.icon, &chan_new.icon)
                            .change("nsfw", &chan_old.nsfw, &chan_new.nsfw)
                            .change("bitrate", &chan_old.bitrate, &chan_new.bitrate)
                            .change("user_limit", &chan_old.user_limit, &chan_new.user_limit)
                            .change("type", &chan_old.ty, &chan_new.ty)
                            .build(),
                    },
                })
                .await?;
        }

        if chan_old.name != chan_new.name {
            // send thread renamed message to thread
            let rename_message_id = data
                .message_create(DbMessageCreate {
                    channel_id: thread_id,
                    attachment_ids: vec![],
                    author_id: user_id,
                    embeds: vec![],
                    message_type: MessageType::ThreadRename(MessageThreadRename {
                        name_new: chan_new.name.clone(),
                        name_old: chan_old.name,
                    }),
                    edited_at: None,
                    created_at: None,
                    mentions: Default::default(),
                })
                .await?;
            let rename_message = data
                .message_get(thread_id, rename_message_id, user_id)
                .await?;
            self.state
                .broadcast_channel(
                    thread_id,
                    user_id,
                    MessageSync::MessageCreate {
                        message: rename_message,
                    },
                )
                .await?;
        }

        let msg = MessageSync::ChannelUpdate {
            channel: Box::new(chan_new.clone()),
        };
        if let Some(room_id) = chan_new.room_id {
            self.state.broadcast_room(room_id, user_id, msg).await?;
        }

        Ok(chan_new)
    }

    pub async fn typing_set(&self, thread_id: ChannelId, user_id: UserId, until: OffsetDateTime) {
        self.typing.insert((thread_id, user_id), until).await;
    }

    pub fn typing_list(&self) -> Vec<(ChannelId, UserId, OffsetDateTime)> {
        self.typing
            .iter()
            .map(|(key, until)| (key.0, key.1, until))
            .collect()
    }
}
