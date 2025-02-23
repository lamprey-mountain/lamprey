use std::sync::Arc;

use moka::future::Cache;
use types::{RoomId, ThreadId, UserId};

use crate::error::Result;
use crate::types::Permissions;
use crate::ServerStateInner;

pub struct ServicePermissions {
    state: Arc<ServerStateInner>,
    cache_perm_room: Cache<(UserId, RoomId), Permissions>,
    cache_perm_thread: Cache<(UserId, RoomId, ThreadId), Permissions>,
}

impl ServicePermissions {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache_perm_room: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            cache_perm_thread: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
        }
    }

    pub async fn for_room(&self, user_id: UserId, room_id: RoomId) -> Result<Permissions> {
        self.cache_perm_room
            .try_get_with((user_id, room_id), async {
                let data = self.state.data();
                let perms = data.permission_room_get(user_id, room_id).await?;
                Result::Ok(perms)
            })
            .await
            .map_err(|err| err.fake_clone())
    }

    pub async fn for_thread(&self, user_id: UserId, thread_id: ThreadId) -> Result<Permissions> {
        let t = self
            .state
            .services()
            .threads
            .get(thread_id, Some(user_id))
            .await?;

        self.cache_perm_thread
            .try_get_with((user_id, t.room_id, thread_id), async {
                let data = self.state.data();
                let perms = data.permission_thread_get(user_id, thread_id).await?;
                Result::Ok(perms)
            })
            .await
            .map_err(|err| err.fake_clone())
    }

    pub async fn invalidate_room(&self, user_id: UserId, room_id: RoomId) {
        self.cache_perm_room.invalidate(&(user_id, room_id)).await;
        self.cache_perm_thread
            .invalidate_entries_if(move |(uid, rid, _), _| room_id == *rid && user_id == *uid)
            .expect("failed to invalidate");
    }

    // might be a good idea to be able to invalidate per role
    pub fn invalidate_room_all(&self, room_id: RoomId) {
        self.cache_perm_room
            .invalidate_entries_if(move |(_, rid), _| room_id == *rid)
            .expect("failed to invalidate");
        self.cache_perm_thread
            .invalidate_entries_if(move |(_, rid, _), _| room_id == *rid)
            .expect("failed to invalidate");
    }

    pub fn invalidate_thread(&self, user_id: UserId, thread_id: ThreadId) {
        self.cache_perm_thread
            .invalidate_entries_if(move |(uid, _, tid), _| thread_id == *tid && user_id == *uid)
            .expect("failed to invalidate");
    }
}
