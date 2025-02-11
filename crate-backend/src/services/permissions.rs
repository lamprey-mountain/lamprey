use std::ops::Deref;
use std::sync::Arc;

use types::{RoomId, ThreadId, UserId};

use crate::error::Result;
use crate::{types::Permissions, ServerState};

pub struct ServicePermissions {
    state: Arc<ServerState>,
}

impl ServicePermissions {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
    }

    pub async fn for_room(&self, user_id: UserId, room_id: RoomId) -> Result<Permissions> {
        if let Some(ent) = self.state.cache_perm_room.get(&(user_id, room_id)) {
            return Ok(ent.deref().clone());
        }
        let data = self.state.data();
        let perms = data.permission_room_get(user_id, room_id).await?;
        self.state
            .cache_perm_room
            .insert((user_id, room_id), perms.clone());
        Ok(perms)
    }

    pub async fn for_thread(&self, user_id: UserId, thread_id: ThreadId) -> Result<Permissions> {
        // TODO: cache this too
        let t = self
            .state
            .data()
            .thread_get(thread_id, Some(user_id))
            .await?;

        if let Some(ent) = self
            .state
            .cache_perm_thread
            .get(&(user_id, t.room_id, thread_id))
        {
            return Ok(ent.deref().clone());
        }
        let data = self.state.data();
        let perms = data.permission_thread_get(user_id, thread_id).await?;
        self.state
            .cache_perm_thread
            .insert((user_id, t.room_id, thread_id), perms.clone());
        Ok(perms)
    }

    pub fn invalidate_room(&self, user_id: UserId, room_id: RoomId) {
        self.state.cache_perm_room.remove(&(user_id, room_id));
        // prob a better way to do this
        self.state
            .cache_perm_thread
            .retain(|(uid, rid, _), _| !(room_id == *rid || user_id == *uid));
    }

    // might be a good idea to be able to invalidate per role
    pub fn invalidate_room_all(&self, room_id: RoomId) {
        self.state
            .cache_perm_room
            .retain(|(_, rid), _| !(room_id == *rid));
        self.state
            .cache_perm_thread
            .retain(|(_, rid, _), _| !(room_id == *rid));
    }
}
