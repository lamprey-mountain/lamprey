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
    cache_is_mutual: Cache<(UserId, UserId), bool>,
}

impl ServicePermissions {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        // not sure what the best way to configure these caches are
        // (userid, roomid) seems a bit inefficient, maybe caching roles would be better
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
            cache_is_mutual: Cache::builder()
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

    // FIXME: cache
    /// check if two users share a common room
    pub async fn is_mutual(&self, a: UserId, b: UserId) -> Result<bool> {
        if a == b {
            return Ok(true);
        }
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        let data = self.state.data();
        self.cache_is_mutual
            .try_get_with((a, b), data.permission_is_mutual(a, b))
            .await
            .map_err(|err| err.fake_clone())
    }

    pub fn invalidate_is_mutual(&self, user_id: UserId) {
        self.cache_is_mutual
            .invalidate_entries_if(move |(a, b), _| *a == user_id || *b == user_id)
            .expect("failed to invalidate");
    }
}
