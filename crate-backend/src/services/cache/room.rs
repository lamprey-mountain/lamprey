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
    pub thread: Channel,

    /// thread members
    pub members: DashMap<UserId, ThreadMember>,
    // maybe include first, last message?
}
