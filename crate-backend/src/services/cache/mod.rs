//! Unified cache for data

use std::sync::Arc;
use std::time::Duration;

use common::v1::types::{
    emoji::EmojiCustom,
    ids::EmojiId,
    preferences::{PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser},
    ChannelId, MessageSync, RoleId, Room, RoomId, RoomMember, User, UserId,
};
use futures::{future::BoxFuture, StreamExt};
use moka::future::Cache;

use crate::{
    error::Result, services::cache::room_actor::RoomActor, types::PaginationQuery, Error,
    ServerStateInner,
};

pub mod permissions;
pub mod room;
pub mod room_actor;
pub mod user;

use crate::services::cache::room::{RoomCommand, RoomHandle, RoomSnapshot, RoomUnavailableReason};
use common::v1::types::error::ApiError;
use common::v1::types::error::ErrorCode;
pub use permissions::PermissionsCalculator;
pub use user::CachedUser;

use dashmap::{DashMap, DashSet};

/// service for caching all in-memory data used by the server
#[derive(Clone)]
pub struct ServiceCache {
    state: Arc<ServerStateInner>,

    // TODO: make not pub?
    pub(crate) rooms: Cache<RoomId, RoomHandle>,

    // TODO: make not pub?
    pub(crate) users: Cache<UserId, User>,

    pub(crate) emojis: Cache<EmojiId, EmojiCustom>,

    /// Keep an in-memory map of UserId -> Set of RoomIds for fast fan-out of presence/user updates
    pub(crate) user_rooms: Arc<DashMap<UserId, DashSet<RoomId>>>,

    // preferences caches
    preferences_global: Cache<UserId, PreferencesGlobal>,
    preferences_room: Cache<(UserId, RoomId), PreferencesRoom>,
    preferences_channel: Cache<(UserId, ChannelId), PreferencesChannel>,
    preferences_user: Cache<(UserId, UserId), PreferencesUser>,
}

impl ServiceCache {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            rooms: Cache::builder()
                .max_capacity(100)
                .support_invalidation_closures()
                .eviction_listener(|room_id, handle: RoomHandle, cause| {
                    tracing::debug!(?room_id, ?cause, "Evicting room actor");
                    let _ = handle.tx.try_send(RoomCommand::Close);
                })
                .build(),
            users: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            emojis: Cache::builder().max_capacity(100_000).build(),
            user_rooms: Arc::new(DashMap::new()),
            preferences_global: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            preferences_room: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            preferences_channel: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            preferences_user: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
        }
    }

    /// register a user as being in a room in the cache
    pub fn member_register(&self, user_id: UserId, room_id: RoomId) {
        self.user_rooms
            .entry(user_id)
            .or_insert_with(DashSet::new)
            .insert(room_id);
    }

    /// unregister a user from a room in the cache
    pub fn member_unregister(&self, user_id: UserId, room_id: RoomId) {
        if let Some(rooms) = self.user_rooms.get(&user_id) {
            rooms.remove(&room_id);
            if rooms.is_empty() {
                drop(rooms);
                self.user_rooms.remove(&user_id);
            }
        }
    }

    pub fn start_background_tasks(&self) {
        let this = self.clone();
        tokio::spawn(async move {
            let mut rx = this.state.subscribe_sushi().await.unwrap();
            while let Some(msg) = rx.next().await {
                this.handle_sync(&msg.message).await;
            }
        });

        let this = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Run every hour
            loop {
                interval.tick().await;
                this.janitor_cleanup().await;
            }
        });
    }

    /// clean up orphaned user_rooms entries
    async fn janitor_cleanup(&self) {
        let mut to_remove = Vec::new();

        for entry in self.user_rooms.iter() {
            let (user_id, rooms) = entry.pair();
            let mut orphaned = Vec::new();

            for room_id in rooms.iter() {
                if !self.rooms.contains_key(&*room_id) {
                    orphaned.push(*room_id);
                }
            }

            for room_id in orphaned {
                rooms.remove(&room_id);
            }

            if rooms.is_empty() {
                to_remove.push(*user_id);
            }
        }

        for user_id in to_remove {
            self.user_rooms.remove(&user_id);
        }
    }

    /// load ALL users
    // TEMP: this is probably horrible for performance
    // this is a bad idea
    pub async fn load_users(&self) -> Result<()> {
        todo!("load all users into cache")
    }

    pub fn load_room(&self, room_id: RoomId) -> BoxFuture<'_, Result<Arc<RoomSnapshot>>> {
        Box::pin(async move {
            let handle = self
                .rooms
                .try_get_with(room_id, async {
                    Ok::<RoomHandle, Error>(RoomActor::spawn(room_id, self.state.clone()))
                })
                .await
                .map_err(|e| e.fake_clone())?;

            let mut rx = handle.snapshot_rx.clone();
            let snapshot = rx
                .wait_for(|s| !s.is_loading())
                .await
                .map_err(|_| Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoom)))?;

            let snapshot = snapshot.clone();

            if snapshot.is_not_found() {
                return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoom)));
            }

            if let RoomSnapshot::Unavailable(_) = snapshot.as_ref() {
                // If it's backlogged, we should probably evict it so it can retry later
                // but for now we just return error.
                return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoom)));
            }

            Ok(snapshot)
        })
    }

    /// mark a room as unavailable
    pub async fn mark_unavailable(&self, room_id: RoomId, _reason: RoomUnavailableReason) {
        if let Some(_handle) = self.rooms.get(&room_id).await {
            // We can't easily update the watch channel from outside the actor
            // unless we have the Sender.
            // For now, we'll just unload it, which is effectively a "retry soon".
            self.unload_room(room_id).await;
        }
    }

    /// unload a single room
    pub async fn unload_room(&self, room_id: RoomId) {
        self.rooms.invalidate(&room_id).await;
    }

    /// update a room's metadata in the cache
    pub async fn update_room(&self, room: Room) {
        if let Some(handle) = self.rooms.get(&room.id).await {
            if let Err(_) = handle
                .tx
                .try_send(RoomCommand::Sync(MessageSync::RoomUpdate {
                    room: room.clone(),
                }))
            {
                self.mark_unavailable(room.id, RoomUnavailableReason::Backlogged)
                    .await;
            }
        }
    }

    /// reload a member from the database and update the cache
    pub async fn reload_member(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        if let Some(handle) = self.rooms.get(&room_id).await {
            let member = self.state.data().room_member_get(room_id, user_id).await?;
            let user = self.state.services().users.get(user_id, None).await?;
            if let Err(_) = handle
                .tx
                .try_send(RoomCommand::Sync(MessageSync::RoomMemberUpdate {
                    member,
                    user,
                }))
            {
                self.mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                    .await;
            }
        }
        Ok(())
    }

    /// remove a member from the cache
    pub async fn remove_member(&self, room_id: RoomId, user_id: UserId) {
        if let Some(handle) = self.rooms.get(&room_id).await {
            if let Err(_) = handle
                .tx
                .try_send(RoomCommand::Sync(MessageSync::RoomMemberDelete {
                    room_id,
                    user_id,
                }))
            {
                self.mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                    .await;
            }
        }
    }

    /// reload a role from the database and update the cache
    pub async fn reload_role(&self, room_id: RoomId, role_id: RoleId) -> Result<()> {
        if let Some(handle) = self.rooms.get(&room_id).await {
            let role = self.state.data().role_select(room_id, role_id).await?;
            if let Err(_) = handle
                .tx
                .try_send(RoomCommand::Sync(MessageSync::RoleUpdate { role }))
            {
                self.mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                    .await;
            }
        }
        Ok(())
    }

    /// remove a role from the cache
    pub async fn remove_role(&self, room_id: RoomId, role_id: RoleId) {
        if let Some(handle) = self.rooms.get(&room_id).await {
            if let Err(_) = handle
                .tx
                .try_send(RoomCommand::Sync(MessageSync::RoleDelete {
                    room_id,
                    role_id,
                }))
            {
                self.mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                    .await;
            }
        }
    }

    /// reload a channel from the database and update the cache
    pub async fn reload_channel(&self, room_id: RoomId, channel_id: ChannelId) -> Result<()> {
        if let Some(handle) = self.rooms.get(&room_id).await {
            let channel = self.state.data().channel_get(channel_id).await?;
            if let Err(_) = handle
                .tx
                .try_send(RoomCommand::Sync(MessageSync::ChannelUpdate {
                    channel: Box::new(channel),
                }))
            {
                self.mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                    .await;
            }
        }
        Ok(())
    }

    /// remove a channel from the cache
    pub async fn remove_channel(&self, room_id: RoomId, _channel_id: ChannelId) {
        if let Some(_handle) = self.rooms.get(&room_id).await {
            // We don't have a direct "Delete" event that only takes ID for channel,
            // but we can send a dummy ChannelUpdate with is_removed = true if needed,
            // or just use unload_room if it's simpler.
            self.unload_room(room_id).await;
        }
    }

    /// reload a thread member from the database and update the cache
    pub async fn reload_thread_member(
        &self,
        room_id: RoomId,
        thread_id: ChannelId,
        user_id: UserId,
    ) -> Result<()> {
        if let Some(handle) = self.rooms.get(&room_id).await {
            let member = self
                .state
                .data()
                .thread_member_get(thread_id, user_id)
                .await?;
            if let Err(_) = handle
                .tx
                .try_send(RoomCommand::Sync(MessageSync::ThreadMemberUpsert {
                    room_id: Some(room_id),
                    thread_id,
                    added: vec![member],
                    removed: vec![],
                }))
            {
                self.mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                    .await;
            }
        }
        Ok(())
    }

    /// remove a thread member from the cache
    pub async fn remove_thread_member(
        &self,
        room_id: RoomId,
        thread_id: ChannelId,
        user_id: UserId,
    ) {
        if let Some(handle) = self.rooms.get(&room_id).await {
            if let Err(_) = handle
                .tx
                .try_send(RoomCommand::Sync(MessageSync::ThreadMemberUpsert {
                    room_id: Some(room_id),
                    thread_id,
                    added: vec![],
                    removed: vec![user_id],
                }))
            {
                self.mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                    .await;
            }
        }
    }

    /// unload all rooms
    pub fn unload_all(&self) {
        self.rooms.invalidate_all();
    }

    /// get a user from the cache, loading from the database if not present
    pub async fn user_get(&self, user_id: UserId) -> Result<User> {
        self.users
            .try_get_with(user_id, self.state.data().user_get(user_id))
            .await
            .map_err(|err| err.fake_clone())
    }

    /// invalidate a user in the cache
    pub async fn user_invalidate(&self, user_id: UserId) {
        self.users.invalidate(&user_id).await;
    }

    /// purge all users from the cache
    pub fn user_purge(&self) {
        self.users.invalidate_all();
    }

    /// get a user's global config from the cache, loading from the database if not present
    pub async fn preferences_get(&self, user_id: UserId) -> Result<PreferencesGlobal> {
        self.preferences_global
            .try_get_with(user_id, self.state.data().preferences_get(user_id))
            .await
            .map_err(|err| err.fake_clone())
    }

    /// invalidate a user's global config in the cache
    pub async fn preferences_invalidate(&self, user_id: UserId) {
        self.preferences_global.invalidate(&user_id).await;
    }

    /// get a user's room config from the cache, loading from the database if not present
    pub async fn preferences_room_get(
        &self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<PreferencesRoom> {
        self.preferences_room
            .try_get_with(
                (user_id, room_id),
                self.state.data().preferences_room_get(user_id, room_id),
            )
            .await
            .map_err(|err| err.fake_clone())
    }

    /// invalidate a user's room config in the cache
    pub async fn preferences_room_invalidate(&self, user_id: UserId, room_id: RoomId) {
        self.preferences_room.invalidate(&(user_id, room_id)).await;
    }

    /// get a user's channel config from the cache, loading from the database if not present
    pub async fn preferences_channel_get(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<PreferencesChannel> {
        self.preferences_channel
            .try_get_with(
                (user_id, channel_id),
                self.state
                    .data()
                    .preferences_channel_get(user_id, channel_id),
            )
            .await
            .map_err(|err| err.fake_clone())
    }

    /// invalidate a user's channel config in the cache
    pub async fn preferences_channel_invalidate(&self, user_id: UserId, channel_id: ChannelId) {
        self.preferences_channel
            .invalidate(&(user_id, channel_id))
            .await;
    }

    /// get a user's config for another user from the cache, loading from the database if not present
    pub async fn preferences_user_get(
        &self,
        user_id: UserId,
        other_id: UserId,
    ) -> Result<PreferencesUser> {
        self.preferences_user
            .try_get_with(
                (user_id, other_id),
                self.state.data().preferences_user_get(user_id, other_id),
            )
            .await
            .map_err(|err| err.fake_clone())
    }

    /// invalidate a user's config for another user in the cache
    pub async fn preferences_user_invalidate(&self, user_id: UserId, other_id: UserId) {
        self.preferences_user.invalidate(&(user_id, other_id)).await;
    }

    /// get an emoji from the cache, loading from the database if not present
    pub async fn emoji_get(&self, emoji_id: EmojiId) -> Result<EmojiCustom> {
        self.emojis
            .try_get_with(emoji_id, self.state.data().emoji_get(emoji_id))
            .await
            .map_err(|err| err.fake_clone())
    }

    /// get multiple emojis from the cache, loading missing ones from the database
    pub async fn emoji_get_many(&self, emoji_ids: &[EmojiId]) -> Result<Vec<EmojiCustom>> {
        if emoji_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut out = Vec::with_capacity(emoji_ids.len());
        let mut missing = Vec::new();

        for id in emoji_ids {
            if let Some(emoji) = self.emojis.get(id).await {
                out.push(emoji);
            } else {
                missing.push(*id);
            }
        }

        if !missing.is_empty() {
            let emojis = self.state.data().emoji_get_many(&missing).await?;
            for emoji in emojis {
                self.emojis.insert(emoji.id, emoji.clone()).await;
                out.push(emoji);
            }
        }

        Ok(out)
    }

    /// invalidate an emoji in the cache
    pub async fn emoji_invalidate(&self, emoji_id: EmojiId) {
        self.emojis.invalidate(&emoji_id).await;
    }

    /// get the permission calculator for this room, loading the room if it doesn't exist
    pub async fn permissions(&self, room_id: RoomId) -> Result<PermissionsCalculator> {
        let snapshot = self.load_room(room_id).await?;
        let data = snapshot
            .get_data()
            .ok_or_else(|| Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoom)))?;
        Ok(PermissionsCalculator {
            room_id,
            owner_id: data.room.owner_id,
            public: data.room.public,
            room: snapshot,
        })
    }

    /// generate an ambient message for a user containing all their initial state
    // PERF: fetch in parallel
    pub async fn generate_ambient_message(&self, user_id: UserId) -> Result<MessageSync> {
        let data = self.state.data();

        let mut room_items = Vec::new();

        // fetch rooms with pagination
        let mut after: Option<RoomId> = None;
        loop {
            let page = data
                .room_list(
                    user_id,
                    PaginationQuery {
                        // TODO: use MAX_ROOM_JOINS
                        // limit: Some(MAX_ROOM_JOINS.try_into().unwrap()),
                        limit: Some(1024),
                        from: after.map(|i| i.into()),
                        ..Default::default()
                    },
                    true,
                )
                .await?;

            if page.items.is_empty() {
                break;
            }

            after = Some(page.items.last().unwrap().id);
            room_items.extend(page.items);

            if !page.has_more {
                break;
            }
        }

        let results = futures::stream::iter(room_items.into_iter())
            .map(|room| {
                let this = self.clone();
                async move {
                    let snapshot = this.load_room(room.id).await?;
                    let member = this
                        .state
                        .data()
                        .room_member_get(room.id, user_id)
                        .await
                        .ok();
                    Ok::<(Room, Arc<RoomSnapshot>, Option<RoomMember>), Error>((
                        room, snapshot, member,
                    ))
                }
            })
            .buffer_unordered(16)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        let mut rooms = Vec::new();
        let mut room_members = Vec::new();
        let mut all_roles = Vec::new();
        let mut all_channels = Vec::new();
        let mut all_threads = Vec::new();

        for (room, snapshot, member) in results {
            let cached_room = snapshot
                .get_data()
                .ok_or_else(|| Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoom)))?;

            if let Some(member) = member {
                room_members.push(member);
            }

            for role in cached_room.roles.values() {
                all_roles.push(role.inner.clone());
            }

            for channel in cached_room.channels.values() {
                all_channels.push(channel.inner.clone());
            }

            for thread in cached_room.threads.values() {
                let thread_inner = &thread.thread;
                if thread_inner.archived_at.is_none() {
                    all_threads.push(thread_inner.clone());
                }
            }

            rooms.push(room);
        }

        // populate private data for all channels
        let srv = self.state.services();
        srv.channels
            .populate_private(&mut all_channels, user_id)
            .await?;
        srv.channels
            .populate_private(&mut all_threads, user_id)
            .await?;

        let config = self.preferences_get(user_id).await?;

        Ok(MessageSync::Ambient {
            user_id,
            rooms,
            roles: all_roles,
            channels: all_channels,
            threads: all_threads,
            room_members,
            config,
        })
    }

    /// update caches from a sync event
    pub async fn handle_sync(&self, event: &MessageSync) {
        if let MessageSync::RoomDelete { room_id } = event {
            self.unload_room(*room_id).await;
            return;
        }

        if let Some(room_id) = event.room_id() {
            if let Some(handle) = self.rooms.get(&room_id).await {
                if let Err(_) = handle.tx.try_send(RoomCommand::Sync(event.clone())) {
                    self.mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                        .await;
                }
                return;
            }
        }

        match event {
            MessageSync::PreferencesGlobal { user_id, config } => {
                self.preferences_global
                    .insert(*user_id, config.clone())
                    .await;
            }
            MessageSync::PreferencesRoom {
                user_id,
                room_id,
                config,
            } => {
                self.preferences_room
                    .insert((*user_id, *room_id), config.clone())
                    .await;
            }
            MessageSync::PreferencesChannel {
                user_id,
                channel_id,
                config,
            } => {
                self.preferences_channel
                    .insert((*user_id, *channel_id), config.clone())
                    .await;
            }
            MessageSync::PreferencesUser {
                user_id,
                target_user_id,
                config,
            } => {
                self.preferences_user
                    .insert((*user_id, *target_user_id), config.clone())
                    .await;
            }
            MessageSync::EmojiCreate { emoji } | MessageSync::EmojiUpdate { emoji } => {
                self.emojis.insert(emoji.id, emoji.clone()).await;
            }
            MessageSync::EmojiDelete { emoji_id, .. } => {
                self.emojis.invalidate(emoji_id).await;
            }
            MessageSync::PresenceUpdate { user_id, presence } => {
                if let Some(mut user) = self.users.get(user_id).await {
                    user.presence = presence.clone();
                    self.users.insert(*user_id, user).await;
                }

                // Find all rooms this user is in and notify their actors
                if let Some(rooms) = self.user_rooms.get(user_id) {
                    for room_id in rooms.iter() {
                        let room_id = *room_id;
                        if let Some(handle) = self.rooms.get(&room_id).await {
                            if let Err(_) = handle.tx.try_send(RoomCommand::Sync(event.clone())) {
                                self.mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                                    .await;
                            }
                        }
                    }
                }
            }
            MessageSync::UserUpdate { user } => {
                self.users.insert(user.id, user.clone()).await;

                // Find all rooms this user is in and notify their actors
                if let Some(rooms) = self.user_rooms.get(&user.id) {
                    for room_id in rooms.iter() {
                        let room_id = *room_id;
                        if let Some(handle) = self.rooms.get(&room_id).await {
                            if let Err(_) = handle.tx.try_send(RoomCommand::Sync(event.clone())) {
                                self.mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                                    .await;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
