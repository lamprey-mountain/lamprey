//! cached/in memory rooms

use common::v1::types::{Channel, ChannelId, Role, RoleId, Room, RoomMember, ThreadMember, UserId};
use dashmap::DashMap;
use tokio::sync::RwLock;

pub struct CachedRoom {
    /// the data of the room itself
    pub inner: RwLock<Room>,

    /// every member in this room
    pub members: DashMap<UserId, RoomMember>,

    /// every non-thread channel in this room
    pub channels: DashMap<ChannelId, Channel>,

    /// all roles in the room
    pub roles: DashMap<RoleId, Role>,

    /// all active threads in the room
    pub threads: DashMap<ChannelId, CachedThread>,
}

pub struct CachedThread {
    /// the thread itself
    pub thread: RwLock<Channel>,

    /// thread members
    pub members: DashMap<UserId, ThreadMember>,
    // maybe include first, last message?
}

impl CachedRoom {
    /// update this room's metadata
    pub async fn room_update(&self, room: Room) {
        let mut inner = self.inner.write().await;
        *inner = room;
    }

    // TODO: move more cache updating stuff here (eg. channel_create, channel_update, channel_delete, role_create, role_update, role_delete)
}
