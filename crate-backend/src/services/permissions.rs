use std::sync::Arc;

use dashmap::DashMap;
use types::{RoomId, ThreadId, UserId};

use crate::error::Result;
use crate::types::Permissions;
use crate::ServerStateInner;

pub struct ServicePermissions {
    state: Arc<ServerStateInner>,
    cache_perm_room: Arc<DashMap<(UserId, RoomId), Permissions>>,
    cache_perm_thread: Arc<DashMap<(UserId, RoomId, ThreadId), Permissions>>,
}

impl ServicePermissions {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache_perm_room: Arc::new(DashMap::new()),
            cache_perm_thread: Arc::new(DashMap::new()),
        }
    }

    pub async fn for_room(&self, user_id: UserId, room_id: RoomId) -> Result<Permissions> {
        if let Some(ent) = self.cache_perm_room.get(&(user_id, room_id)) {
            return Ok(ent.to_owned());
        }
        let data = self.state.data();
        let perms = data.permission_room_get(user_id, room_id).await?;
        self.cache_perm_room
            .insert((user_id, room_id), perms.clone());
        Ok(perms)
    }

    pub async fn for_thread(&self, user_id: UserId, thread_id: ThreadId) -> Result<Permissions> {
        let t = self
            .state
            .services()
            .threads
            .get(thread_id, Some(user_id))
            .await?;

        if let Some(ent) = self.cache_perm_thread.get(&(user_id, t.room_id, thread_id)) {
            return Ok(ent.to_owned());
        }

        let data = self.state.data();
        let perms = data.permission_thread_get(user_id, thread_id).await?;
        self.cache_perm_thread
            .insert((user_id, t.room_id, thread_id), perms.clone());
        Ok(perms)
    }

    pub fn invalidate_room(&self, user_id: UserId, room_id: RoomId) {
        self.cache_perm_room.remove(&(user_id, room_id));
        // prob a better way to do this
        self.cache_perm_thread
            .retain(|(uid, rid, _), _| !(room_id == *rid || user_id == *uid));
    }

    // might be a good idea to be able to invalidate per role
    pub fn invalidate_room_all(&self, room_id: RoomId) {
        self.cache_perm_room.retain(|(_, rid), _| room_id != *rid);
        self.cache_perm_thread
            .retain(|(_, rid, _), _| room_id != *rid);
    }
}
