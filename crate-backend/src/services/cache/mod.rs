//! Unified cache for data

use std::sync::Arc;

use common::v1::types::{
    emoji::EmojiCustom,
    ids::EmojiId,
    preferences::{PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser},
    ChannelId, MessageSync, RoleId, Room, RoomId, User, UserId,
};
use dashmap::DashMap;
use futures::{future::BoxFuture, StreamExt};
use moka::future::Cache;
use tokio::sync::RwLock;
use tracing::warn;

use crate::{
    error::Result,
    types::{PaginationQuery, PermissionBits},
    ServerStateInner,
};

pub mod permissions;
pub mod room;
pub mod user;

pub use permissions::PermissionsCalculator;
pub use room::{
    CachedChannel, CachedPermissionOverwrite, CachedRole, CachedRoom, CachedRoomMember,
    CachedThread,
};
pub use user::CachedUser;

/// service for caching all in-memory data used by the server
#[derive(Clone)]
pub struct ServiceCache {
    state: Arc<ServerStateInner>,

    // TODO: make not pub?
    pub(crate) rooms: Cache<RoomId, Arc<CachedRoom>>,

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
            rooms: Cache::builder().max_capacity(100).build(),
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

    // NOTE: i probably want to shard this later
    pub fn load_room(&self, room_id: RoomId) -> BoxFuture<'_, Result<Arc<CachedRoom>>> {
        Box::pin(async move {
            self.rooms
                .try_get_with(room_id, async {
                    self.load_room_inner(room_id).await.map(Arc::new)
                })
                .await
                .map_err(|e| e.fake_clone())
        })
    }

    async fn load_room_inner(&self, room_id: RoomId) -> Result<CachedRoom> {
        let data = self.state.data();
        let srv = self.state.services();

        // 1. load room
        let room = data.room_get(room_id).await?;

        // 2. load members
        let room_members = data.room_member_list_all(room_id).await?;
        let members = DashMap::new();
        for member in room_members {
            // PERF: use get_many
            let user = srv.users.get(member.user_id, None).await?;
            members.insert(
                member.user_id,
                CachedRoomMember {
                    member,
                    user: Arc::new(user),
                },
            );
        }

        // 3. load roles
        let roles_data = data
            .role_list(
                room_id,
                PaginationQuery {
                    limit: Some(1024),
                    ..Default::default()
                },
            )
            .await?
            .items;

        let roles = DashMap::new();
        for role in roles_data {
            let allow = PermissionBits::from(&role.allow);
            let deny = PermissionBits::from(&role.deny);
            roles.insert(
                role.id,
                CachedRole {
                    inner: role,
                    allow,
                    deny,
                },
            );
        }

        // 4. load channels
        let channels_data = data.channel_list(room_id).await?;
        let channels = DashMap::new();
        for channel in channels_data {
            if channel.ty.is_thread() {
                continue;
            }
            let overwrites = channel
                .permission_overwrites
                .iter()
                .map(|ow| CachedPermissionOverwrite {
                    id: ow.id,
                    ty: ow.ty,
                    allow: PermissionBits::from(&ow.allow),
                    deny: PermissionBits::from(&ow.deny),
                })
                .collect();
            channels.insert(
                channel.id,
                CachedChannel {
                    inner: channel,
                    overwrites,
                },
            );
        }

        // 5. load active threads and members
        let active_threads_vec = data.thread_all_active_room(room_id).await?;
        let threads = DashMap::new();
        for thread in active_threads_vec {
            let thread_members_vec = data.thread_member_list_all(thread.id).await?;
            let members_map = DashMap::new();
            for member in thread_members_vec {
                members_map.insert(member.user_id, member);
            }
            threads.insert(
                thread.id,
                CachedThread {
                    thread: RwLock::new(thread),
                    members: members_map,
                },
            );
        }

        let cached_room = CachedRoom {
            inner: RwLock::new(room),
            members,
            channels,
            roles,
            threads,
        };

        Ok(cached_room)
    }

    /// unload a single room
    pub async fn unload_room(&self, room_id: RoomId) {
        self.rooms.invalidate(&room_id).await;
    }

    /// update a room's metadata in the cache
    pub async fn update_room(&self, room: Room) {
        if let Some(cached) = self.rooms.get(&room.id).await {
            let mut inner = cached.inner.write().await;
            *inner = room;
        }
    }

    /// reload a member from the database and update the cache
    pub async fn reload_member(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        if let Some(cached) = self.rooms.get(&room_id).await {
            let member = self.state.data().room_member_get(room_id, user_id).await?;
            let user = self.state.services().users.get(user_id, None).await?;
            cached.members.insert(
                user_id,
                CachedRoomMember {
                    member,
                    user: Arc::new(user),
                },
            );
        }
        Ok(())
    }

    /// remove a member from the cache
    pub async fn remove_member(&self, room_id: RoomId, user_id: UserId) {
        if let Some(cached) = self.rooms.get(&room_id).await {
            cached.members.remove(&user_id);
        }
    }

    /// reload a role from the database and update the cache
    pub async fn reload_role(&self, room_id: RoomId, role_id: RoleId) -> Result<()> {
        if let Some(cached) = self.rooms.get(&room_id).await {
            let role = self.state.data().role_select(room_id, role_id).await?;
            let allow = PermissionBits::from(&role.allow);
            let deny = PermissionBits::from(&role.deny);
            cached.roles.insert(
                role_id,
                CachedRole {
                    inner: role,
                    allow,
                    deny,
                },
            );
        }
        Ok(())
    }

    /// remove a role from the cache
    pub async fn remove_role(&self, room_id: RoomId, role_id: RoleId) {
        if let Some(cached) = self.rooms.get(&room_id).await {
            cached.roles.remove(&role_id);
        }
    }

    /// reload a channel from the database and update the cache
    pub async fn reload_channel(&self, room_id: RoomId, channel_id: ChannelId) -> Result<()> {
        if let Some(cached) = self.rooms.get(&room_id).await {
            let channel = self.state.data().channel_get(channel_id).await?;
            if channel.ty.is_thread() {
                let thread_members_vec =
                    self.state.data().thread_member_list_all(channel_id).await?;
                let members_map = DashMap::new();
                for member in thread_members_vec {
                    members_map.insert(member.user_id, member);
                }
                cached.threads.insert(
                    channel_id,
                    CachedThread {
                        thread: RwLock::new(channel.clone()),
                        members: members_map,
                    },
                );
            } else {
                let overwrites = channel
                    .permission_overwrites
                    .iter()
                    .map(|ow| CachedPermissionOverwrite {
                        id: ow.id,
                        ty: ow.ty,
                        allow: PermissionBits::from(&ow.allow),
                        deny: PermissionBits::from(&ow.deny),
                    })
                    .collect();
                cached.channels.insert(
                    channel_id,
                    CachedChannel {
                        inner: channel,
                        overwrites,
                    },
                );
            }
        }
        Ok(())
    }

    /// remove a channel from the cache
    pub async fn remove_channel(&self, room_id: RoomId, channel_id: ChannelId) {
        if let Some(cached) = self.rooms.get(&room_id).await {
            cached.channels.remove(&channel_id);
            cached.threads.remove(&channel_id);
        }
    }

    /// reload a thread member from the database and update the cache
    pub async fn reload_thread_member(
        &self,
        room_id: RoomId,
        thread_id: ChannelId,
        user_id: UserId,
    ) -> Result<()> {
        if let Some(cached) = self.rooms.get(&room_id).await {
            if let Some(thread) = cached.threads.get(&thread_id) {
                let member = self
                    .state
                    .data()
                    .thread_member_get(thread_id, user_id)
                    .await?;
                thread.members.insert(user_id, member);
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
        if let Some(cached) = self.rooms.get(&room_id).await {
            if let Some(thread) = cached.threads.get(&thread_id) {
                thread.members.remove(&user_id);
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
        Ok(self.load_room(room_id).await?.permissions().await)
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

                for entry in cached_room.roles.iter() {
                    all_roles.push(entry.value().inner.clone());
                }

                for entry in cached_room.channels.iter() {
                    all_channels.push(entry.value().inner.clone());
                }

                for entry in cached_room.threads.iter() {
                    let thread = entry.value();
                    let thread_inner = thread.thread.read().await;
                    if thread_inner.archived_at.is_none() {
                        all_threads.push(thread_inner.clone());
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
        match event {
            MessageSync::RoomUpdate { room } => {
                self.update_room(room.clone()).await;
            }
            MessageSync::RoomDelete { room_id } => {
                self.unload_room(*room_id).await;
            }
            MessageSync::MessageCreate { message } => {
                let Some(room_id) = message.room_id else {
                    return;
                };
                if let Some(cached) = self.rooms.get(&room_id).await {
                    if let Some(thread) = cached.threads.get(&message.channel_id) {
                        let mut t = thread.thread.write().await;
                        t.last_version_id = Some(message.latest_version.version_id);
                        t.last_message_id = Some(message.id);
                    } else if let Some(mut channel) = cached.channels.get_mut(&message.channel_id) {
                        channel.inner.last_version_id = Some(message.latest_version.version_id);
                        channel.inner.last_message_id = Some(message.id);
                    }
                }
            }
            MessageSync::MessageUpdate { message } => {
                let Some(room_id) = message.room_id else {
                    return;
                };
                if let Some(cached) = self.rooms.get(&room_id).await {
                    if let Some(thread) = cached.threads.get(&message.channel_id) {
                        let mut t = thread.thread.write().await;
                        t.last_version_id = Some(message.latest_version.version_id);
                    } else if let Some(mut channel) = cached.channels.get_mut(&message.channel_id) {
                        channel.inner.last_version_id = Some(message.latest_version.version_id);
                    }
                }
            }
            MessageSync::ChannelCreate { channel } => {
                let Some(room_id) = channel.room_id else {
                    return;
                };
                if let Some(cached) = self.rooms.get(&room_id).await {
                    let overwrites = channel
                        .permission_overwrites
                        .iter()
                        .map(|ow| CachedPermissionOverwrite {
                            id: ow.id,
                            ty: ow.ty,
                            allow: PermissionBits::from(&ow.allow),
                            deny: PermissionBits::from(&ow.deny),
                        })
                        .collect();
                    if channel.ty.is_thread() {
                        cached.threads.insert(
                            channel.id,
                            CachedThread {
                                thread: RwLock::new(*channel.to_owned()),
                                members: DashMap::new(),
                            },
                        );
                    } else {
                        cached.channels.insert(
                            channel.id,
                            CachedChannel {
                                inner: *channel.clone(),
                                overwrites,
                            },
                        );
                    }
                }
            }
            MessageSync::ChannelUpdate { channel } => {
                let Some(room_id) = channel.room_id else {
                    return;
                };
                if let Some(cached) = self.rooms.get(&room_id).await {
                    let overwrites = channel
                        .permission_overwrites
                        .iter()
                        .map(|ow| CachedPermissionOverwrite {
                            id: ow.id,
                            ty: ow.ty,
                            allow: PermissionBits::from(&ow.allow),
                            deny: PermissionBits::from(&ow.deny),
                        })
                        .collect();
                    if channel.ty.is_thread() {
                        if channel.deleted_at.is_some() {
                            cached.threads.remove(&channel.id);
                        } else if let Some(thread) = cached.threads.get(&channel.id) {
                            let mut t = thread.thread.write().await;
                            *t = *channel.to_owned();
                        }
                    } else if channel.deleted_at.is_some() {
                        cached.channels.remove(&channel.id);
                    } else {
                        cached.channels.insert(
                            channel.id,
                            CachedChannel {
                                inner: *channel.clone(),
                                overwrites,
                            },
                        );
                    }
                }
            }
            MessageSync::RoleCreate { role } => {
                if let Some(room) = self.rooms.get(&role.room_id).await {
                    let allow = PermissionBits::from(&role.allow);
                    let deny = PermissionBits::from(&role.deny);
                    room.roles.insert(
                        role.id,
                        CachedRole {
                            inner: role.to_owned(),
                            allow,
                            deny,
                        },
                    );
                }
            }
            MessageSync::RoleUpdate { role } => {
                if let Some(room) = self.rooms.get(&role.room_id).await {
                    let allow = PermissionBits::from(&role.allow);
                    let deny = PermissionBits::from(&role.deny);
                    if room
                        .roles
                        .insert(
                            role.id,
                            CachedRole {
                                inner: role.to_owned(),
                                allow,
                                deny,
                            },
                        )
                        .is_none()
                    {
                        warn!(room_id = ?role.room_id, role_id = ?role.id, "got RoleUpdate for role that does not exist");
                    }
                }
            }
            MessageSync::RoleDelete { room_id, role_id } => {
                self.remove_role(*room_id, *role_id).await;
                if let Some(room) = self.rooms.get(room_id).await {
                    for mut member in room.members.iter_mut() {
                        member.member.roles.retain(|r| r != role_id);
                    }
                }
                self.state.services().perms.invalidate_user_ranks(*room_id);
            }
            MessageSync::RoleReorder { room_id, roles } => {
                if let Some(room) = self.rooms.get(room_id).await {
                    for item in roles {
                        if let Some(mut role) = room.roles.get_mut(&item.role_id) {
                            role.value_mut().inner.position = item.position;
                        }
                    }
                }
            }
            MessageSync::RoomMemberCreate { member, user }
            | MessageSync::RoomMemberUpdate { member, user } => {
                if let Some(room) = self.rooms.get(&member.room_id).await {
                    let cached_member = CachedRoomMember {
                        member: member.clone(),
                        user: Arc::new(user.clone()),
                    };
                    room.members.insert(member.user_id, cached_member);
                }
            }
            MessageSync::RoomMemberDelete { room_id, user_id } => {
                self.remove_member(*room_id, *user_id).await;
            }
            MessageSync::ThreadMemberUpsert {
                thread_id,
                added,
                removed,
                ..
            } => {
                let srv = self.state.services();
                if let Ok(chan) = srv.channels.get(*thread_id, None).await {
                    if let Some(room_id) = chan.room_id {
                        if let Some(room) = self.rooms.get(&room_id).await {
                            if let Some(thread) = room.threads.get(&thread_id) {
                                for member in added {
                                    thread.members.insert(member.user_id, member.clone());
                                }

                                for user_id in removed {
                                    thread.members.remove(&user_id);
                                }
                            }
                        }
                    }
                }
            }
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
            _ => {}
        }
    }
}
