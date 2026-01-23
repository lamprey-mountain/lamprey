//! Unified cache for data

use std::sync::Arc;

use common::v1::types::{ChannelId, MessageSync, RoleId, Room, RoomId, UserId};
use dashmap::DashMap;
use moka::future::Cache;
use tokio::sync::RwLock;
use tracing::warn;

use crate::{
    error::Result,
    services::cache::room::{CachedRoom, CachedThread},
    types::PaginationQuery,
    ServerStateInner,
};

mod permissions;
mod room;
mod user;

use permissions::PermissionsCalculator;

/// service for loading and storing data used by the server
// NOTE: do i really want to be using dashmap everywhere?
pub struct ServiceCache {
    state: Arc<ServerStateInner>,
    rooms: Cache<RoomId, Arc<CachedRoom>>,
    // users: DashMap<UserId, User>,
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
        }
    }

    /// load ALL users
    // TEMP: this is probably horrible for performance
    // this is a bad idea
    pub async fn load_users(&self) -> Result<()> {
        todo!("load all users into cache")
    }

    // NOTE: i probably want to shard this later
    pub async fn load_room(&self, room_id: RoomId) -> Result<Arc<CachedRoom>> {
        self.rooms
            .try_get_with(room_id, async {
                self.load_room_inner(room_id).await.map(Arc::new)
            })
            .await
            .map_err(|e| e.fake_clone())
    }

    async fn load_room_inner(&self, room_id: RoomId) -> Result<CachedRoom> {
        let data = self.state.data();

        // 1. load room
        let room = data.room_get(room_id).await?;

        // 2. load members
        let room_members = data.room_member_list_all(room_id).await?;
        let members = DashMap::new();
        for member in room_members {
            members.insert(member.user_id, member);
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
            roles.insert(role.id, role);
        }

        // 4. load channels
        let channels_data = data.channel_list(room_id).await?;
        let channels = DashMap::new();
        for channel in channels_data {
            channels.insert(channel.id, channel);
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
            cached.members.insert(user_id, member);
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
            cached.roles.insert(role_id, role);
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
                        thread: RwLock::new(channel),
                        members: members_map,
                    },
                );
            } else {
                cached.channels.insert(channel_id, channel);
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

    /// get the permission calculator for this room, loading the room if it doesn't exist
    pub async fn permissions(&self, room_id: RoomId) -> Result<PermissionsCalculator> {
        Ok(self.load_room(room_id).await?.permissions().await)
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
            MessageSync::ChannelCreate { channel } => {
                let Some(room_id) = channel.room_id else {
                    return;
                };
                if let Some(cached) = self.rooms.get(&room_id).await {
                    if channel.ty.is_thread() {
                        cached.threads.insert(
                            channel.id,
                            CachedThread {
                                thread: RwLock::new(*channel.to_owned()),
                                members: DashMap::new(),
                            },
                        );
                    } else {
                        cached.channels.insert(channel.id, *channel.clone());
                    }
                }
            }
            MessageSync::ChannelUpdate { channel } => {
                let Some(room_id) = channel.room_id else {
                    return;
                };
                if let Some(cached) = self.rooms.get(&room_id).await {
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
                        cached.channels.insert(channel.id, *channel.clone());
                    }
                }
            }
            MessageSync::RoleCreate { role } => {
                if let Some(room) = self.rooms.get(&role.room_id).await {
                    room.roles.insert(role.id, role.clone());
                }
            }
            MessageSync::RoleUpdate { role } => {
                if let Some(room) = self.rooms.get(&role.room_id).await {
                    if room.roles.insert(role.id, role.clone()).is_none() {
                        warn!(room_id = ?role.room_id, role_id = ?role.id, "got RoleUpdate for role that does not exist");
                    }
                }
            }
            MessageSync::RoleDelete { room_id, role_id } => {
                self.remove_role(*room_id, *role_id).await;
            }
            MessageSync::RoleReorder { room_id, roles } => {
                if let Some(room) = self.rooms.get(room_id).await {
                    for item in roles {
                        if let Some(mut role) = room.roles.get_mut(&item.role_id) {
                            role.position = item.position;
                        }
                    }
                }
            }
            MessageSync::RoomMemberCreate { member }
            | MessageSync::RoomMemberUpdate { member }
            | MessageSync::RoomMemberUpsert { member } => {
                if let Some(room) = self.rooms.get(&member.room_id).await {
                    room.members.insert(member.user_id, member.clone());
                }
            }
            MessageSync::RoomMemberDelete { room_id, user_id } => {
                self.remove_member(*room_id, *user_id).await;
            }
            MessageSync::ThreadMemberUpsert { member } => {
                let srv = self.state.services();
                if let Ok(chan) = srv.channels.get(member.thread_id, None).await {
                    if let Some(room_id) = chan.room_id {
                        if let Some(room) = self.rooms.get(&room_id).await {
                            if let Some(thread) = room.threads.get(&member.thread_id) {
                                thread.members.insert(member.user_id, member.clone());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
