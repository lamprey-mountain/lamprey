//! Unified cache for data

use std::sync::Arc;
use std::time::Duration;

use crate::{error::Result, types::PaginationQuery, Error, ServerStateInner};
use common::v1::types::{
    emoji::EmojiCustom,
    ids::EmojiId,
    preferences::{PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser},
    ChannelId, MessageSync, Room, RoomId, RoomMember, User, UserId,
};
use futures::{future::BoxFuture, StreamExt};
use moka::future::Cache;

pub mod permissions;

pub use crate::services::rooms::{
    CachedChannel, CachedRole, CachedRoomMember, CachedThread, RoomCommand, RoomHandle,
    RoomSnapshot, RoomUnavailableReason,
};

use common::v1::types::error::ApiError;
use common::v1::types::error::ErrorCode;
pub use permissions::PermissionsCalculator;

/// service for caching all in-memory data used by the server
#[derive(Clone)]
pub struct ServiceCache {
    state: Arc<ServerStateInner>,

    // TODO: make not pub?
    pub(crate) users: Cache<UserId, User>,

    pub(crate) emojis: Cache<EmojiId, EmojiCustom>,

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
            users: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            emojis: Cache::builder().max_capacity(100_000).build(),
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
        self.state
            .services()
            .rooms
            .member_register(user_id, room_id);
    }

    /// unregister a user from a room in the cache
    pub fn member_unregister(&self, user_id: UserId, room_id: RoomId) {
        self.state
            .services()
            .rooms
            .member_unregister(user_id, room_id);
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
        let rooms_srv = self.state.services().rooms.clone();
        let actors = rooms_srv.actors.clone();

        for entry in rooms_srv.user_rooms.iter() {
            let (user_id, user_rooms) = entry.pair();
            let mut orphaned = Vec::new();

            for room_id in user_rooms.iter() {
                if !actors.contains_key(&*room_id) {
                    orphaned.push(*room_id);
                }
            }

            for room_id in orphaned {
                user_rooms.remove(&room_id);
            }

            if user_rooms.is_empty() {
                to_remove.push(*user_id);
            }
        }

        for user_id in to_remove {
            rooms_srv.user_rooms.remove(&user_id);
        }
    }

    pub fn load_room(
        &self,
        room_id: RoomId,
        ensure_members: bool,
    ) -> BoxFuture<'_, Result<Arc<RoomSnapshot>>> {
        let rooms = self.state.services().rooms.clone();
        Box::pin(async move { rooms.load_room(room_id, ensure_members).await })
    }

    /// mark a room as unavailable
    pub async fn mark_unavailable(&self, room_id: RoomId, reason: RoomUnavailableReason) {
        self.state
            .services()
            .rooms
            .mark_unavailable(room_id, reason)
            .await;
    }

    /// unload a single room
    pub async fn unload_room(&self, room_id: RoomId) {
        self.state.services().rooms.unload_cache(room_id).await;
    }

    /// update a room's metadata in the cache
    pub async fn update_room(&self, room: Room) {
        self.state.services().rooms.update_cache(room).await;
    }

    /// reload a member from the database and update the cache
    pub async fn reload_member(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        self.state
            .services()
            .rooms
            .reload_member(room_id, user_id)
            .await
    }

    /// reload a channel from the database and update the cache
    pub async fn reload_channel(&self, room_id: RoomId, channel_id: ChannelId) -> Result<()> {
        self.state
            .services()
            .rooms
            .reload_channel(room_id, channel_id)
            .await
    }

    /// unload all rooms
    pub fn unload_all(&self) {
        self.state.services().rooms.unload_all_cache();
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
    pub async fn permissions(
        &self,
        room_id: RoomId,
        ensure_members: bool,
    ) -> Result<PermissionsCalculator> {
        let snapshot = self.load_room(room_id, ensure_members).await?;
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
                    let snapshot = this.load_room(room.id, true).await?;
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
            let rooms = self.state.services().rooms.clone();
            if let Some(handle) = rooms.actors.get(&room_id).await {
                if let Err(_) = handle.tx.try_send(RoomCommand::Sync(event.clone())) {
                    rooms
                        .mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
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
                let rooms_srv = self.state.services().rooms.clone();
                let rooms_to_notify = if let Some(rooms_set) = rooms_srv.user_rooms.get(user_id) {
                    rooms_set.iter().map(|r| *r).collect::<Vec<_>>()
                } else {
                    Vec::new()
                };

                for room_id in rooms_to_notify {
                    if let Some(handle) = rooms_srv.actors.get(&room_id).await {
                        if let Err(_) = handle.tx.try_send(RoomCommand::Sync(event.clone())) {
                            rooms_srv
                                .mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                                .await;
                        }
                    }
                }
            }
            MessageSync::UserUpdate { user } => {
                self.users.insert(user.id, user.clone()).await;

                // Find all rooms this user is in and notify their actors
                let rooms_srv = self.state.services().rooms.clone();
                let rooms_to_notify = if let Some(rooms_set) = rooms_srv.user_rooms.get(&user.id) {
                    rooms_set.iter().map(|r| *r).collect::<Vec<_>>()
                } else {
                    Vec::new()
                };

                for room_id in rooms_to_notify {
                    if let Some(handle) = rooms_srv.actors.get(&room_id).await {
                        if let Err(_) = handle.tx.try_send(RoomCommand::Sync(event.clone())) {
                            rooms_srv
                                .mark_unavailable(room_id, RoomUnavailableReason::Backlogged)
                                .await;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
