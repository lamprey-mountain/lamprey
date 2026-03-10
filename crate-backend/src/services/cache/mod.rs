//! Unified cache for data

use std::sync::Arc;

use common::v1::types::{
    emoji::EmojiCustom,
    ids::EmojiId,
    preferences::{PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser},
    ChannelId, MessageSync, RoleId, Room, RoomId, User, UserId,
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

use crate::services::cache::room::{RoomCommand, RoomHandle, RoomSnapshot, RoomStatus};
use common::v1::types::error::ApiError;
use common::v1::types::error::ErrorCode;
pub use permissions::PermissionsCalculator;
pub use user::CachedUser;

/// service for caching all in-memory data used by the server
#[derive(Clone)]
pub struct ServiceCache {
    state: Arc<ServerStateInner>,

    // TODO: make not pub?
    pub(crate) rooms: Cache<RoomId, RoomHandle>,

    // TODO: make not pub?
    pub(crate) users: Cache<UserId, User>,

    pub(crate) emojis: Cache<EmojiId, EmojiCustom>,

    // preferences caches
    preferences_global: Cache<UserId, PreferencesGlobal>,
    preferences_room: Cache<(UserId, RoomId), PreferencesRoom>,
    preferences_channel: Cache<(UserId, ChannelId), PreferencesChannel>,
    preferences_user: Cache<(UserId, UserId), PreferencesUser>,
    // presences: DashMap<UserId, Presence>,
    // TODO: more caching?
    // - dm/gdm channels?
    // - voice states?
    // - voice calls?
    // - session data?
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

    pub fn start_background_tasks(&self) {
        let this = self.clone();
        tokio::spawn(async move {
            let mut rx = this.state.subscribe_sushi().await.unwrap();
            while let Some(msg) = rx.next().await {
                this.handle_sync(&msg.message).await;
            }
        });
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

            let mut snapshot_rx = handle.snapshot.clone();
            loop {
                {
                    let snapshot = snapshot_rx.borrow_and_update();
                    match snapshot.status {
                        RoomStatus::Ready => return Ok(Arc::clone(&snapshot)),
                        RoomStatus::NotFound => {
                            return Err(Error::ApiError(ApiError::from_code(
                                ErrorCode::UnknownRoom,
                            )))
                        }
                        RoomStatus::Loading => {}
                    }
                }
                snapshot_rx
                    .changed()
                    .await
                    .map_err(|_| Error::Internal("room actor died".to_string()))?;
            }
        })
    }

    /// unload a single room
    pub async fn unload_room(&self, room_id: RoomId) {
        self.rooms.invalidate(&room_id).await;
    }

    /// update a room's metadata in the cache
    pub async fn update_room(&self, room: Room) {
        if let Some(handle) = self.rooms.get(&room.id).await {
            let _ = handle
                .tx
                .send(RoomCommand::Sync(MessageSync::RoomUpdate { room }))
                .await;
        }
    }

    /// reload a member from the database and update the cache
    pub async fn reload_member(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        if let Some(handle) = self.rooms.get(&room_id).await {
            let member = self.state.data().room_member_get(room_id, user_id).await?;
            let user = self.state.services().users.get(user_id, None).await?;
            let _ = handle
                .tx
                .send(RoomCommand::Sync(MessageSync::RoomMemberUpdate {
                    member,
                    user,
                }))
                .await;
        }
        Ok(())
    }

    /// remove a member from the cache
    pub async fn remove_member(&self, room_id: RoomId, user_id: UserId) {
        if let Some(handle) = self.rooms.get(&room_id).await {
            let _ = handle
                .tx
                .send(RoomCommand::Sync(MessageSync::RoomMemberDelete {
                    room_id,
                    user_id,
                }))
                .await;
        }
    }

    /// reload a role from the database and update the cache
    pub async fn reload_role(&self, room_id: RoomId, role_id: RoleId) -> Result<()> {
        if let Some(handle) = self.rooms.get(&room_id).await {
            let role = self.state.data().role_select(room_id, role_id).await?;
            let _ = handle
                .tx
                .send(RoomCommand::Sync(MessageSync::RoleUpdate { role }))
                .await;
        }
        Ok(())
    }

    /// remove a role from the cache
    pub async fn remove_role(&self, room_id: RoomId, role_id: RoleId) {
        if let Some(handle) = self.rooms.get(&room_id).await {
            let _ = handle
                .tx
                .send(RoomCommand::Sync(MessageSync::RoleDelete {
                    room_id,
                    role_id,
                }))
                .await;
        }
    }

    /// reload a channel from the database and update the cache
    pub async fn reload_channel(&self, room_id: RoomId, channel_id: ChannelId) -> Result<()> {
        if let Some(handle) = self.rooms.get(&room_id).await {
            let channel = self.state.data().channel_get(channel_id).await?;
            let _ = handle
                .tx
                .send(RoomCommand::Sync(MessageSync::ChannelUpdate {
                    channel: Box::new(channel),
                }))
                .await;
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
            let _ = handle
                .tx
                .send(RoomCommand::Sync(MessageSync::ThreadMemberUpsert {
                    room_id: Some(room_id),
                    thread_id,
                    added: vec![member],
                    removed: vec![],
                }))
                .await;
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
            let _ = handle
                .tx
                .send(RoomCommand::Sync(MessageSync::ThreadMemberUpsert {
                    room_id: Some(room_id),
                    thread_id,
                    added: vec![],
                    removed: vec![user_id],
                }))
                .await;
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
        Ok(PermissionsCalculator {
            room_id,
            owner_id: snapshot.room.owner_id,
            public: snapshot.room.public,
            room: snapshot,
        })
    }

    /// generate an ambient message for a user containing all their initial state
    // PERF: fetch in parallel
    pub async fn generate_ambient_message(&self, user_id: UserId) -> Result<MessageSync> {
        let data = self.state.data();

        let mut rooms = Vec::new();
        let mut room_members = Vec::new();
        let mut all_roles = Vec::new();
        let mut all_channels = Vec::new();
        let mut all_threads = Vec::new();

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

            for room in page.items {
                let cached_room = self.load_room(room.id).await?;

                if let Ok(member) = data.room_member_get(room.id, user_id).await {
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
                        all_threads.push(thread_inner.as_ref().clone());
                    }
                }

                rooms.push(room);
            }

            if !page.has_more {
                break;
            }
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
            if let Some(handle) = self.rooms.get(room_id).await {
                let _ = handle.tx.send(RoomCommand::Sync(event.clone())).await;
            }
            return;
        }

        if let Some(room_id) = event.room_id() {
            if let Some(handle) = self.rooms.get(&room_id).await {
                let _ = handle.tx.send(RoomCommand::Sync(event.clone())).await;
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
            MessageSync::PresenceUpdate { user_id, .. }
            | MessageSync::UserUpdate {
                user: User { id: user_id, .. },
            } => {
                // Find all rooms this user is in and notify their actors
                if let Ok(rooms) = self.state.data().room_list_user_all(*user_id).await {
                    for room_id in rooms {
                        if let Some(handle) = self.rooms.get(&room_id).await {
                            let _ = handle.tx.send(RoomCommand::Sync(event.clone())).await;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
