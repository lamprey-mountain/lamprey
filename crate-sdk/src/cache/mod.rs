use std::{collections::HashMap, sync::Arc};

use common::{
    v1::types::{Channel, Message, Relationship, Role, Room, RoomMember, ThreadMember, User},
    v2::types::{ChannelId, MessageId, RoleId, RoomId, UserId},
};

mod permissions;
mod settings;

pub use permissions::{PermissionsCalculator, PermissionsError};
pub use settings::{CacheBuilder, CacheSettings};

// TODO: custom debug impl for Cache

#[derive(Clone)]
pub struct Cache {
    pub(crate) inner: Arc<CacheInner>,
}

pub struct CacheInner {
    pub(crate) settings: CacheSettings,
    pub(crate) rooms: HashMap<RoomId, CachedRoom>,
    pub(crate) channels: HashMap<ChannelId, CachedChannel>,
    pub(crate) users: HashMap<UserId, CachedUser>,
    // TODO: use LruCache
    // pub(crate) users: lru::LruCache<UserId, CachedUser>,
}

#[derive(Debug, Clone)]
pub struct CachedRoom {
    pub inner: Room,
    pub members: HashMap<UserId, RoomMember>,
    pub channels: HashMap<ChannelId, CachedChannel>, // contains threads
    pub roles: HashMap<RoleId, Role>,
}

#[derive(Debug, Clone)]
pub struct CachedUser {
    pub inner: User,

    /// your relationship with this user, if it is known
    pub relationship: Option<Relationship>,
    // TODO: use this instead of inner.presence?
    // pub presence: Option<Presence>,
}

#[derive(Debug, Clone)]
pub struct CachedChannel {
    pub inner: Channel,
    pub members: HashMap<UserId, ThreadMember>,
    pub messages: HashMap<MessageId, Message>,
    pub ranges: HashMap<(), ()>,
    // messages: lru::LruCache<MessageId, Arc<Message>>,
    // ranges: lru::LruCache<MessageId, Arc<Message>>,
    // messages: Arc<RwLock<MessagesInner>>,
}

pub struct CacheRef<'a, T> {
    inner: &'a T,
}

impl<'a, T> std::ops::Deref for CacheRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

pub type CachedRoomRef<'a> = CacheRef<'a, CachedRoom>;
pub type CachedUserRef<'a> = CacheRef<'a, CachedUser>;
pub type CachedChannelRef<'a> = CacheRef<'a, CachedChannel>;

pub struct CacheStats {
    pub rooms: usize,
    pub channels: usize,
    pub users: usize,
}

impl Cache {
    pub fn builder() -> CacheBuilder {
        CacheBuilder::default()
    }

    /// get a reference to a room from its id
    pub fn room(&self, id: RoomId) -> Option<CachedRoomRef<'_>> {
        self.inner.rooms.get(&id).map(|r| CacheRef { inner: r })
    }

    /// get a reference to a channel from its id
    pub fn channel(&self, id: ChannelId) -> Option<CachedChannelRef<'_>> {
        self.inner.channels.get(&id).map(|c| CacheRef { inner: c })
    }

    /// get a reference to a user from its id
    pub fn user(&self, id: UserId) -> Option<CachedUserRef<'_>> {
        self.inner.users.get(&id).map(|u| CacheRef { inner: u })
    }

    /// iterate over all cached rooms
    pub fn rooms(&self) -> impl Iterator<Item = RoomId> {
        self.inner.rooms.keys().copied()
    }

    /// iterate over all cached channels
    pub fn channels(&self) -> impl Iterator<Item = ChannelId> {
        self.inner.channels.keys().copied()
    }

    /// iterate over all cached users
    pub fn users(&self) -> impl Iterator<Item = UserId> {
        self.inner.users.keys().copied()
    }

    /// get cache stats
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            rooms: self.inner.rooms.len(),
            channels: self.inner.channels.len(),
            users: self.inner.users.len(),
        }
    }
}
