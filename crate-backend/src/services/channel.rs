use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use common::v1::types::presence::Status;
use common::v1::types::util::{Changes, Diff, Time};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, Channel, ChannelCreate, ChannelId,
    ChannelPatch, ChannelType, MessageChannelIcon, MessageSync, MessageThreadRename, MessageType,
    PaginationQuery, Permission, PermissionOverwrite, RoomId, ThreadMemberPut, User, UserId,
    SERVER_USER_ID,
};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use moka::future::Cache;
use time::OffsetDateTime;
use tracing::warn;

use crate::error::{Error, Result};
use crate::types::{DbChannelCreate, DbChannelPrivate, DbMessageCreate};
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

            let thread_member = if thread.ty.is_thread() {
                self.state
                    .data()
                    .thread_member_get(channel_id, user_id)
                    .await
                    .ok()
                    .map(Box::new)
            } else {
                None
            };

            thread = Channel {
                recipients,
                is_unread: Some(private_data.is_unread),
                last_read_id: private_data.last_read_id.map(Into::into),
                mention_count: Some(private_data.mention_count as u64),
                user_config: Some(user_config),
                thread_member,
                ..thread
            }
        }

        let members = self.state.data().thread_member_list_all(channel_id).await?;

        let mut online_count = 0;
        for member in members {
            if self.state.services().presence.get(member.user_id).status != Status::Offline {
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
                if self.state.services().presence.get(member.user_id).status != Status::Offline {
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

    pub fn purge_cache(&self) {
        self.cache_thread.invalidate_all();
        self.cache_thread_private.invalidate_all();
        self.cache_thread_recipients.invalidate_all();
    }

    pub async fn create_channel(
        &self,
        user_id: UserId,
        room_id: Option<RoomId>,
        reason: Option<String>,
        json: ChannelCreate,
    ) -> Result<Channel> {
        let srv = self.state.services();
        let data = self.state.data();
        let perms = if let Some(parent_id) = json.parent_id {
            srv.perms.for_channel(user_id, parent_id).await?
        } else if let Some(room_id) = room_id {
            srv.perms.for_room(user_id, room_id).await?
        } else {
            return Err(Error::BadStatic(
                "Channel must have a parent or be in a room",
            ));
        };
        perms.ensure(Permission::ViewChannel)?;

        let parent_id_opt = json.parent_id;

        match json.ty {
            ChannelType::Text
            | ChannelType::Announcement
            | ChannelType::Forum
            | ChannelType::Forum2
            | ChannelType::Voice
            | ChannelType::Broadcast
            | ChannelType::Category
            | ChannelType::Calendar
            | ChannelType::Ticket
            | ChannelType::Info
            | ChannelType::Wiki => {
                perms.ensure(Permission::ChannelManage)?;
            }
            ChannelType::ThreadPublic => {
                let parent_id = json
                    .parent_id
                    .ok_or(Error::BadStatic("threads must have a parent channel"))?;
                let parent = srv.channels.get(parent_id, Some(user_id)).await?;
                if !parent.ty.has_public_threads() {
                    return Err(Error::BadStatic(
                        "public threads can only be created in specific channel types",
                    ));
                }
                perms.ensure(Permission::ThreadCreatePublic)?;

                if !perms.can_bypass_slowmode() {
                    if let Some(parent_id) = parent_id_opt {
                        if let Some(thread_slowmode_expire_at) = data
                            .channel_get_thread_slowmode_expire_at(parent_id, user_id)
                            .await?
                        {
                            if thread_slowmode_expire_at > Time::now_utc() {
                                return Err(Error::BadStatic("slowmode in effect"));
                            }
                        }

                        if let Some(slowmode_delay) = parent.slowmode_thread {
                            let next_thread_time =
                                Time::now_utc() + std::time::Duration::from_secs(slowmode_delay);
                            data.channel_set_thread_slowmode_expire_at(
                                parent_id,
                                user_id,
                                next_thread_time,
                            )
                            .await?;
                        }
                    }
                }
            }
            ChannelType::ThreadForum2 => {
                let parent_id = json
                    .parent_id
                    .ok_or(Error::BadStatic("threads must have a parent channel"))?;
                let parent = srv.channels.get(parent_id, Some(user_id)).await?;
                if !parent.ty.has_forum2_threads() {
                    return Err(Error::BadStatic(
                        "forum2 threads can only be created in forum2 channels",
                    ));
                }
                perms.ensure(Permission::ThreadCreatePublic)?;

                if !perms.can_bypass_slowmode() {
                    if let Some(parent_id) = parent_id_opt {
                        if let Some(thread_slowmode_expire_at) = data
                            .channel_get_thread_slowmode_expire_at(parent_id, user_id)
                            .await?
                        {
                            if thread_slowmode_expire_at > Time::now_utc() {
                                return Err(Error::BadStatic("slowmode in effect"));
                            }
                        }

                        if let Some(slowmode_delay) = parent.slowmode_thread {
                            let next_thread_time =
                                Time::now_utc() + std::time::Duration::from_secs(slowmode_delay);
                            data.channel_set_thread_slowmode_expire_at(
                                parent_id,
                                user_id,
                                next_thread_time,
                            )
                            .await?;
                        }
                    }
                }
            }
            ChannelType::ThreadPrivate => {
                let parent_id = json
                    .parent_id
                    .ok_or(Error::BadStatic("threads must have a parent channel"))?;
                let parent = srv.channels.get(parent_id, Some(user_id)).await?;
                if !parent.ty.has_private_threads() {
                    return Err(Error::BadStatic(
                        "threads can only be created in specific channel types",
                    ));
                }
                perms.ensure(Permission::ThreadCreatePrivate)?;

                if !perms.can_bypass_slowmode() {
                    if let Some(parent_id) = parent_id_opt {
                        if let Some(thread_slowmode_expire_at) = data
                            .channel_get_thread_slowmode_expire_at(parent_id, user_id)
                            .await?
                        {
                            if thread_slowmode_expire_at > Time::now_utc() {
                                return Err(Error::BadStatic("slowmode in effect"));
                            }
                        }

                        if let Some(slowmode_delay) = parent.slowmode_thread {
                            let next_thread_time =
                                Time::now_utc() + std::time::Duration::from_secs(slowmode_delay);
                            data.channel_set_thread_slowmode_expire_at(
                                parent_id,
                                user_id,
                                next_thread_time,
                            )
                            .await?;
                        }
                    }
                }
            }
            ChannelType::Dm | ChannelType::Gdm => {
                return Err(Error::BadStatic(
                    "can't create a direct message thread in a room",
                ))
            }
            ChannelType::Document => {
                if let Some(parent_id) = json.parent_id {
                    let parent = srv.channels.get(parent_id, Some(user_id)).await?;
                    if parent.ty == ChannelType::Wiki {
                        perms.ensure(Permission::DocumentCreate)?;
                    } else {
                        perms.ensure(Permission::ChannelManage)?;
                    }
                } else {
                    perms.ensure(Permission::ChannelManage)?;
                }
            }
            ChannelType::DocumentComment => {
                perms.ensure(Permission::DocumentComment)?;
            }
        };
        if json.bitrate.is_some_and(|b| b > 393216) {
            return Err(Error::BadStatic("bitrate is too high"));
        }
        if !json.ty.has_voice() && json.bitrate.is_some() {
            return Err(Error::BadStatic("cannot set bitrate for non voice channel"));
        }
        if !json.ty.has_voice() && json.user_limit.is_some() {
            return Err(Error::BadStatic(
                "cannot set user_limit for non voice channel",
            ));
        }
        if !json.ty.has_icon() && json.icon.is_some() {
            return Err(Error::BadStatic("this channel type cannot have an icon"));
        }
        if !json.ty.has_url() && json.url.is_some() {
            return Err(Error::BadStatic("cannot set url for non info channel"));
        }

        if json.default_auto_archive_duration.is_some() && !json.ty.has_threads() {
            return Err(Error::BadStatic("channel does not have threads"));
        }

        if json.auto_archive_duration.is_some() && !json.ty.is_thread() {
            return Err(Error::BadStatic(
                "auto_archive_duration can only be set on threads",
            ));
        }

        if let Some(icon) = json.icon {
            let media = data.media_select(icon).await?;
            if !matches!(
                media.inner.source.info,
                common::v1::types::MediaTrackInfo::Image(_)
            ) {
                return Err(Error::BadStatic("media not an image"));
            }
        }

        if let Some(tags) = &json.tags {
            if !json.ty.is_taggable() {
                return Err(Error::BadStatic("channel type is not taggable"));
            }

            let parent_id = json.parent_id.ok_or(Error::BadStatic(
                "threads must have a parent channel to have tags",
            ))?;

            let forum_channel = self.get(parent_id, None).await?;
            let available_tags = forum_channel.tags_available.unwrap_or_default();

            let available_tags_map: HashMap<_, _> =
                available_tags.iter().map(|t| (t.id, t)).collect();

            // check permissions for each tag
            for tag_id in tags {
                let Some(tag) = available_tags_map.get(tag_id) else {
                    return Err(Error::BadStatic("invalid tag for this forum"));
                };

                if tag.restricted {
                    if !perms.has(Permission::ThreadEdit) && !perms.has(Permission::ThreadManage) {
                        return Err(Error::BadStatic(
                            "missing permission to apply restricted tag",
                        ));
                    }
                }
            }
        }

        let channel_id = data
            .channel_create(DbChannelCreate {
                room_id: room_id.map(|id| id.into_inner()),
                creator_id: user_id,
                name: json.name.clone(),
                description: json.description.clone(),
                ty: match json.ty {
                    ChannelType::Dm | ChannelType::Gdm => {
                        // this should be unreachable due to the check above
                        warn!("unreachable: dm/gdm thread creation in room");
                        return Err(Error::BadStatic(
                            "can't create a direct message thread in a room",
                        ));
                    }
                    ty => ty.into(),
                },
                nsfw: json.nsfw,
                bitrate: json.bitrate.map(|b| b as i32),
                user_limit: json.user_limit.map(|u| u as i32),
                parent_id: json.parent_id.map(|i| *i),
                owner_id: None,
                icon: json.icon.map(|i| *i),
                invitable: json.invitable,
                auto_archive_duration: json.auto_archive_duration.map(|d| d as i64),
                default_auto_archive_duration: json.default_auto_archive_duration.map(|d| d as i64),
                slowmode_thread: json.slowmode_thread.map(|d| d as i64),
                slowmode_message: json.slowmode_message.map(|d| d as i64),
                default_slowmode_message: json.default_slowmode_message.map(|d| d as i64),
                tags: json.tags,
                url: json.url,
            })
            .await?;

        if let Some(icon) = json.icon {
            data.media_link_create_exclusive(
                icon,
                *channel_id,
                crate::types::MediaLinkType::ChannelIcon,
            )
            .await?;
        }

        for overwrite in json.permission_overwrites {
            data.permission_overwrite_upsert(
                channel_id,
                overwrite.id,
                overwrite.ty,
                overwrite.allow,
                overwrite.deny,
            )
            .await?;
        }

        data.thread_member_put(channel_id, user_id, ThreadMemberPut {})
            .await?;
        let thread_member = data.thread_member_get(channel_id, user_id).await?;

        let channel = srv.channels.get(channel_id, Some(user_id)).await?;
        if let Some(room_id) = room_id {
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
                            .add("parent_id", &channel.parent_id)
                            .add("url", &channel.url)
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
        } else if let Some(parent_id) = json.parent_id {
            self.state
                .broadcast_channel(
                    parent_id,
                    user_id,
                    MessageSync::ChannelCreate {
                        channel: Box::new(channel.clone()),
                    },
                )
                .await?;
        }

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
            let can_unarchive = patch.archived == Some(false);
            let mut other_changes = patch.clone();
            other_changes.archived = None;
            let has_other_changes = other_changes.changes(&chan_old);

            if !can_unarchive || has_other_changes {
                return Err(Error::BadStatic("thread is archived"));
            }
        }
        if chan_old.deleted_at.is_some() {
            return Err(Error::BadStatic("thread is removed"));
        }

        perms.ensure_unlocked()?;

        // if the patch contains more than just archive/lock changes, do a general permission check
        let mut other_changes = patch.clone();
        other_changes.archived = None;
        other_changes.locked = None;
        if other_changes.changes(&chan_old) {
            if chan_old.ty.is_thread() {
                if chan_old.creator_id != user_id {
                    perms.ensure(Permission::ThreadEdit)?;
                }
            } else {
                perms.ensure(Permission::ChannelEdit)?;
            }
        }

        if patch
            .auto_archive_duration
            .changes(&chan_old.auto_archive_duration)
            || patch
                .default_auto_archive_duration
                .changes(&chan_old.default_auto_archive_duration)
            || patch.slowmode_thread.changes(&chan_old.slowmode_thread)
            || patch.slowmode_message.changes(&chan_old.slowmode_message)
            || patch
                .default_slowmode_message
                .changes(&chan_old.default_slowmode_message)
        {
            if !perms.has(Permission::ThreadManage) && !perms.has(Permission::ChannelManage) {
                return Err(Error::MissingPermissions);
            }
        }

        // shortcut if it wont modify the thread
        if !patch.changes(&chan_old) {
            return Ok(chan_old);
        }

        if let Some(new_ty) = patch.ty {
            if !chan_old.ty.can_change_to(new_ty) {
                return Err(Error::BadStatic("invalid channel type change"));
            }

            if chan_old.ty.is_thread() && new_ty.is_thread() && chan_old.ty != new_ty {
                perms.ensure(Permission::ThreadManage)?;
            }
        }

        if let Some(new_parent_id_opt) = patch.parent_id {
            if new_parent_id_opt != chan_old.parent_id {
                if let Some(old_parent_id) = chan_old.parent_id {
                    let old_parent_perms = srv.perms.for_channel(user_id, old_parent_id).await?;
                    old_parent_perms.ensure(Permission::ThreadManage)?;
                }

                if let Some(new_parent_id) = new_parent_id_opt {
                    let new_parent_perms = srv.perms.for_channel(user_id, new_parent_id).await?;
                    new_parent_perms.ensure(Permission::ThreadManage)?;
                }
            }
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
        if !chan_old.ty.has_url() && patch.url.is_some() {
            return Err(Error::BadStatic("cannot set url for non info channel"));
        }

        if patch
            .archived
            .is_some_and(|a| a != chan_old.archived_at.is_some())
        {
            if !chan_old.ty.is_thread() {
                return Err(Error::BadStatic("not a thread"));
            }
            data.thread_member_get(thread_id, user_id).await?;
        }

        if patch.locked.as_ref().is_some_and(|a| a != &chan_old.locked) {
            if chan_old.ty.is_thread() {
                perms.ensure(Permission::ThreadLock)?;
            } else {
                perms.ensure(Permission::ChannelManage)?;
            }
        }

        if let Some(Some(icon)) = patch.icon {
            if chan_old.ty.has_icon() {
                return Err(Error::BadStatic("this channel doesnt have an icon"));
            }
            let media = data.media_select(icon).await?;
            if !matches!(
                media.inner.source.info,
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

            // Enforce restricted tags
            let old_tags: HashSet<_> = chan_old
                .tags
                .as_ref()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect();
            let new_tags: HashSet<_> = tags.iter().cloned().collect();
            let added_tags = new_tags.difference(&old_tags);

            for added_tag_id in added_tags {
                let tag = available_tags
                    .iter()
                    .find(|t| t.id == *added_tag_id)
                    .unwrap();
                if tag.restricted {
                    if !perms.has(Permission::ThreadEdit) && !perms.has(Permission::ThreadManage) {
                        return Err(Error::BadStatic(
                            "missing permission to apply restricted tag",
                        ));
                    }
                }
            }
        }

        if patch.default_auto_archive_duration.is_some() && !chan_old.ty.has_threads() {
            return Err(Error::BadStatic("channel does not have threads"));
        }

        if patch.auto_archive_duration.is_some() && !chan_old.ty.is_thread() {
            return Err(Error::BadStatic(
                "auto_archive_duration can only be set on threads",
            ));
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
                            .change("url", &chan_old.url, &chan_new.url)
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
                            .change("parent_id", &chan_old.parent_id, &chan_new.parent_id)
                            .change("invitable", &chan_old.invitable, &chan_new.invitable)
                            .change(
                                "auto_archive_duration",
                                &chan_old.auto_archive_duration,
                                &chan_new.auto_archive_duration,
                            )
                            .change(
                                "default_auto_archive_duration",
                                &chan_old.default_auto_archive_duration,
                                &chan_new.default_auto_archive_duration,
                            )
                            .change(
                                "slowmode_thread",
                                &chan_old.slowmode_thread,
                                &chan_new.slowmode_thread,
                            )
                            .change(
                                "slowmode_message",
                                &chan_old.slowmode_message,
                                &chan_new.slowmode_message,
                            )
                            .change(
                                "default_slowmode_message",
                                &chan_old.default_slowmode_message,
                                &chan_new.default_slowmode_message,
                            )
                            .build(),
                    },
                })
                .await?;
        }

        if chan_old.name != chan_new.name {
            // send thread renamed message to thread
            let rename_message_id = data
                .message_create(DbMessageCreate {
                    id: None,
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
                    removed_at: None,
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

        if chan_old.icon != chan_new.icon {
            let icon_message_id = data
                .message_create(DbMessageCreate {
                    id: None,
                    channel_id: thread_id,
                    attachment_ids: vec![],
                    author_id: user_id,
                    embeds: vec![],
                    message_type: MessageType::ChannelIcon(MessageChannelIcon {
                        icon_id_old: chan_old.icon,
                        icon_id_new: chan_new.icon,
                    }),
                    edited_at: None,
                    created_at: None,
                    removed_at: None,
                    mentions: Default::default(),
                })
                .await?;
            let icon_message = data
                .message_get(thread_id, icon_message_id, user_id)
                .await?;
            self.state
                .broadcast_channel(
                    thread_id,
                    user_id,
                    MessageSync::MessageCreate {
                        message: icon_message,
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

    pub fn start_background_tasks(&self) {
        tokio::spawn(Self::spawn_auto_archive_task(self.state.clone()));
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

    pub async fn spawn_auto_archive_task(state: Arc<ServerStateInner>) {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let data = state.data();
            let srv = state.services();

            match data.thread_auto_archive().await {
                Ok(archived_thread_ids) => {
                    if !archived_thread_ids.is_empty() {
                        tracing::info!("auto-archived {} threads", archived_thread_ids.len());

                        for thread_id in archived_thread_ids {
                            srv.channels.invalidate(thread_id).await;

                            if let Ok(channel) = srv.channels.get(thread_id, None).await {
                                if let Some(room_id) = channel.room_id {
                                    let msg = MessageSync::ChannelUpdate {
                                        channel: Box::new(channel),
                                    };
                                    let _ =
                                        state.broadcast_room(room_id, SERVER_USER_ID, msg).await;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("failed to auto-archive threads: {}", e);
                }
            }
        }
    }

    /// fetch the full list of permission overwrites from topmost parent channel to this current channel
    pub async fn fetch_overwrite_ancestors(
        &self,
        channel_id: ChannelId,
    ) -> Result<Vec<Vec<PermissionOverwrite>>> {
        // TODO: optimize
        let srv = self.state.services();
        let mut top = self.get(channel_id, None).await?;
        let mut overwrites = vec![top.permission_overwrites.clone()];
        while let Some(parent_id) = top.parent_id {
            let chan = srv.channels.get(parent_id, None).await?;
            overwrites.push(chan.permission_overwrites.clone());
            top = chan;
        }
        overwrites.reverse();
        Ok(overwrites)
    }
}
