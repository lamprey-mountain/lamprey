use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use common::v1::types::util::{Changes, Diff};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Channel, ChannelCreate, ChannelId,
    ChannelPatch, ChannelType, MessageSync, MessageThreadRename, MessageType, PaginationQuery,
    Permission, RoomId, ThreadMemberPut, User, UserId,
};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use moka::future::Cache;
use time::OffsetDateTime;
use tracing::warn;

use crate::error::{Error, Result};
use crate::types::{DbChannelCreate, DbChannelPrivate, DbChannelType, DbMessageCreate};
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
                            PaginationQuery {
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
                mention_count: Some(private_data.mention_count as u64),
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

    pub async fn get_many(
        &self,
        channel_ids: &[ChannelId],
        user_id: Option<UserId>,
    ) -> Result<Vec<Channel>> {
        if channel_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut channels = self.state.data().channel_get_many(channel_ids).await?;
        if let Some(user_id) = user_id {
            for channel in &mut channels {
                let channel_id = channel.id;
                let private_data = self
                    .cache_thread_private
                    .try_get_with(
                        (channel.id, user_id),
                        self.state.data().channel_get_private(channel_id, user_id),
                    )
                    .await
                    .map_err(|err| err.fake_clone())?;

                let state = self.state.clone();
                let thread_ty = channel.ty;
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
                                PaginationQuery {
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
                let recipients: Vec<_> =
                    recipients.into_iter().filter(|u| u.id != user_id).collect();

                let user_config = self
                    .state
                    .data()
                    .user_config_channel_get(user_id, channel_id)
                    .await?;

                channel.recipients = recipients;
                channel.is_unread = Some(private_data.is_unread);
                channel.last_read_id = private_data.last_read_id.map(Into::into);
                channel.mention_count = Some(private_data.mention_count as u64);
                channel.user_config = Some(user_config);
            }
        }

        for channel in &mut channels {
            let members = self.state.data().thread_member_list_all(channel.id).await?;
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
            channel.online_count = online_count;
        }

        Ok(channels)
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

    pub async fn create_channel(
        &self,
        user_id: UserId,
        room_id: RoomId,
        reason: Option<String>,
        json: ChannelCreate,
    ) -> Result<Channel> {
        let srv = self.state.services();
        let data = self.state.data();
        let perms = if let Some(parent_id) = json.parent_id {
            srv.perms.for_channel(user_id, parent_id).await?
        } else {
            srv.perms.for_room(user_id, room_id).await?
        };
        perms.ensure(Permission::ViewChannel)?;
        match json.ty {
            ChannelType::Text | ChannelType::Forum | ChannelType::Voice | ChannelType::Category => {
                perms.ensure(Permission::ChannelManage)?;
            }
            ChannelType::ThreadPublic => {
                let parent_id = json
                    .parent_id
                    .ok_or(Error::BadStatic("threads must have a parent channel"))?;
                let parent = srv.channels.get(parent_id, Some(user_id)).await?;
                if !matches!(parent.ty, ChannelType::Text | ChannelType::Forum) {
                    return Err(Error::BadStatic(
                        "threads can only be created in text or forum channels",
                    ));
                }
                perms.ensure(Permission::ThreadCreatePublic)?;
            }
            ChannelType::ThreadPrivate => {
                let parent_id = json
                    .parent_id
                    .ok_or(Error::BadStatic("threads must have a parent channel"))?;
                let parent = srv.channels.get(parent_id, Some(user_id)).await?;
                if !matches!(parent.ty, ChannelType::Text | ChannelType::Forum) {
                    return Err(Error::BadStatic(
                        "threads can only be created in text or forum channels",
                    ));
                }
                perms.ensure(Permission::ThreadCreatePrivate)?;
            }
            ChannelType::Calendar => return Err(Error::BadStatic("not yet implemented")),
            ChannelType::Dm | ChannelType::Gdm => {
                return Err(Error::BadStatic(
                    "can't create a direct message thread in a room",
                ))
            }
        };
        if json.bitrate.is_some_and(|b| b > 393216) {
            return Err(Error::BadStatic("bitrate is too high"));
        }
        if json.ty != ChannelType::Voice && json.bitrate.is_some() {
            return Err(Error::BadStatic("cannot set bitrate for non voice thread"));
        }
        if json.ty != ChannelType::Voice && json.user_limit.is_some() {
            return Err(Error::BadStatic(
                "cannot set user_limit for non voice thread",
            ));
        }
        let channel_id = data
            .channel_create(DbChannelCreate {
                room_id: Some(room_id.into_inner()),
                creator_id: user_id,
                name: json.name.clone(),
                description: json.description.clone(),
                ty: match json.ty {
                    ChannelType::Text => DbChannelType::Text,
                    ChannelType::Forum => DbChannelType::Forum,
                    ChannelType::Voice => DbChannelType::Voice,
                    ChannelType::Category => DbChannelType::Category,
                    ChannelType::ThreadPublic => DbChannelType::ThreadPublic,
                    ChannelType::ThreadPrivate => DbChannelType::ThreadPrivate,
                    ChannelType::Calendar => return Err(Error::BadStatic("not yet implemented")),
                    ChannelType::Dm | ChannelType::Gdm => {
                        // this should be unreachable due to the check above
                        warn!("unreachable: dm/gdm thread creation in room");
                        return Err(Error::BadStatic(
                            "can't create a direct message thread in a room",
                        ));
                    }
                },
                nsfw: json.nsfw,
                bitrate: json.bitrate.map(|b| b as i32),
                user_limit: json.user_limit.map(|u| u as i32),
                parent_id: json.parent_id.map(|i| *i),
                owner_id: None,
                icon: None,
            })
            .await?;

        data.thread_member_put(channel_id, user_id, ThreadMemberPut {})
            .await?;
        let thread_member = data.thread_member_get(channel_id, user_id).await?;

        let channel = srv.channels.get(channel_id, Some(user_id)).await?;
        self.state
            .audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id,
                session_id: None,
                reason: reason.clone(),
                ty: AuditLogEntryType::ChannelCreate {
                    channel_id,
                    channel_type: channel.ty,
                    changes: Changes::new()
                        .add("name", &channel.name)
                        .add("description", &channel.description)
                        .add("nsfw", &channel.nsfw)
                        .add("user_limit", &channel.user_limit)
                        .add("bitrate", &channel.bitrate)
                        .add("type", &channel.ty)
                        .build(),
                },
            })
            .await?;

        self.state
            .broadcast_room(
                room_id,
                user_id,
                MessageSync::ChannelCreate {
                    channel: Box::new(channel.clone()),
                },
            )
            .await?;
        self.state
            .broadcast_channel(
                channel.id,
                user_id,
                MessageSync::ThreadMemberUpsert {
                    member: thread_member,
                },
            )
            .await?;

        Ok(channel)
    }

    pub async fn update(
        &self,
        user_id: UserId,
        thread_id: ChannelId,
        patch: ChannelPatch,
        reason: Option<String>,
    ) -> Result<Channel> {
        // check update perms
        let perms = self
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

        // FIXME: don't require ThreadEdit or ChannelEdit permissions to archive/lock threads
        if chan_old.ty.is_thread() {
            if chan_old.creator_id != user_id {
                perms.ensure(Permission::ThreadEdit)?;
            }
        } else {
            perms.ensure(Permission::ChannelEdit)?;
        }

        // shortcut if it wont modify the thread
        if !patch.changes(&chan_old) {
            return Ok(chan_old);
        }

        if patch.bitrate.is_some_and(|b| b.is_some_and(|b| b > 393216)) {
            return Err(Error::BadStatic("bitrate is too high"));
        }
        if !chan_old.ty.has_voice() && patch.bitrate.is_some() {
            return Err(Error::BadStatic("cannot set bitrate for non voice thread"));
        }
        if !chan_old.ty.has_voice() && patch.user_limit.is_some() {
            return Err(Error::BadStatic(
                "cannot set user_limit for non voice thread",
            ));
        }

        if patch
            .archived
            .is_some_and(|a| a != chan_old.archived_at.is_some())
        {
            if !chan_old.ty.is_thread() {
                return Err(Error::BadStatic("not a thread"));
            }
            if chan_old.creator_id != user_id {
                perms.ensure(Permission::ThreadManage)?;
            }
        }

        if patch.locked.is_some_and(|a| a != chan_old.locked) {
            if chan_old.ty.is_thread() {
                perms.ensure(Permission::ThreadLock)?;
            } else {
                perms.ensure(Permission::ChannelManage)?;
            }
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

        if let Some(tags) = &patch.tags {
            if !chan_old.ty.is_taggable() {
                return Err(Error::BadStatic("channel is not taggable"));
            }
            perms.ensure(Permission::TagApply)?;
            // check if all tags are valid for this forum
            let forum_id = chan_old
                .parent_id
                .ok_or(Error::BadStatic("thread has no parent forum"))?;

            let forum_channel = self.get(forum_id, None).await?;
            let available_tags = forum_channel.tags_available.unwrap_or_default();

            let available_tag_ids: HashSet<_> = available_tags.iter().map(|t| t.id).collect();

            for tag_id in tags {
                if !available_tag_ids.contains(tag_id) {
                    return Err(Error::BadStatic("invalid tag for this forum"));
                }
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
                            .change("type", &chan_old.ty, &chan_new.ty)
                            .change("name", &chan_old.name, &chan_new.name)
                            .change("description", &chan_old.description, &chan_new.description)
                            .change("icon", &chan_old.icon, &chan_new.icon)
                            .change("nsfw", &chan_old.nsfw, &chan_new.nsfw)
                            .change("bitrate", &chan_old.bitrate, &chan_new.bitrate)
                            .change("user_limit", &chan_old.user_limit, &chan_new.user_limit)
                            .change(
                                "archived",
                                &chan_old.archived_at.is_some(),
                                &chan_new.archived_at.is_some(),
                            )
                            .change("locked", &chan_old.locked, &chan_new.locked)
                            .change("tags", &chan_old.tags, &chan_new.tags)
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

    /// get all channels a user can see that are in rooms, along with whether the user has the ThreadManage permission. does not include dm channels
    pub async fn list_user_room_channels(&self, user_id: UserId) -> Result<Vec<(ChannelId, bool)>> {
        let rooms = self
            .state
            .data()
            .room_list(
                user_id,
                PaginationQuery {
                    from: None,
                    to: None,
                    dir: None,
                    limit: Some(1024),
                },
                false,
            )
            .await?;
        let mut out = vec![];
        for room in rooms.items {
            out.extend(
                self.list_user_room_channels_single(user_id, room.id)
                    .await?,
            );
        }
        Ok(out)
    }

    /// like list_user_room_channels, but only for a single room
    pub async fn list_user_room_channels_single(
        &self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<Vec<(ChannelId, bool)>> {
        let channels = self
            .state
            .data()
            .channel_list(
                room_id,
                user_id,
                PaginationQuery {
                    from: None,
                    to: None,
                    dir: None,
                    limit: Some(1024),
                },
                None,
            )
            .await?;
        let mut out = vec![];
        for ch in channels.items {
            let p = self
                .state
                .services()
                .perms
                .for_channel(user_id, ch.id)
                .await?;
            if p.has(Permission::ViewChannel) {
                out.push((ch.id, p.has(Permission::ThreadManage)));
            }
        }
        Ok(out)
    }
}
