//! Unified cache for data

use std::sync::Arc;

use common::v1::types::{ids::SERVER_USER_ID, MessageSync, RoomId};
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
        let user_id = room.owner_id.unwrap_or(SERVER_USER_ID);
        let channels = DashMap::new();
        let mut cursor = None;
        loop {
            // TODO: fetch all channels in one query
            // theres a hard limit of 1024 channels per room, but channel_list is kinda broken
            let page = data
                .channel_list(
                    room_id,
                    user_id,
                    PaginationQuery {
                        from: cursor,
                        limit: Some(1024),
                        ..Default::default()
                    },
                    None,
                )
                .await?;

            let Some(last_id) = page.items.last().map(|c| c.id) else {
                break;
            };

            for channel in page.items {
                if !channel.ty.is_thread() {
                    channels.insert(channel.id, channel);
                }
            }

            // NOTE: may be redundant, considering that last_id would have been None if there are no more channels
            if page.has_more {
                cursor = Some(last_id);
            } else {
                break;
            }
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
                    thread,
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

    /// unload all rooms
    pub fn unload_all(&self) {
        self.rooms.invalidate_all();
    }

    /// get the permission calculator for this room, loading the room if it doesn't exist
    pub async fn permissions(&self, room_id: RoomId) -> Result<PermissionsCalculator> {
        let room = self.load_room(room_id).await?;
        let inner = room.inner.read().await;
        let owner_id = inner.owner_id;
        let public = inner.public;
        drop(inner);
        Ok(PermissionsCalculator {
            room_id,
            owner_id,
            public,
            room,
        })
    }

    /// update caches from a sync event
    pub async fn handle_sync(&self, event: &MessageSync) {
        match event {
            // TODO: handle other sync events
            MessageSync::RoleUpdate { role } => {
                let Some(room) = self.rooms.get(&role.room_id).await else {
                    return;
                };

                if room.roles.insert(role.id, role.clone()).is_none() {
                    warn!(room_id = ?role.room_id, role_id = ?role.id, "got RoleUpdate for role that does not exist");
                }
            }
            _ => todo!(),
        }
    }
}
