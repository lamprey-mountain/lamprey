use std::{collections::HashMap, sync::Arc};

use common::{
    v1::types::{
        Channel, PermissionBits, PermissionOverwrite, Relationship, Role, Room, RoomMember,
        ThreadMember, User,
    },
    v2::types::{ChannelId, RoleId, RoomId, UserId},
};

mod config;
mod permissions;

pub use config::{CacheBuilder, CacheConfig};
pub use permissions::RoomPermissions;
use tokio::sync::RwLock;

use crate::messages::MessagesInner;

#[derive(Debug, Default, Clone)]
pub struct Cache {
    pub(crate) inner: Arc<CacheInner>,
}

#[derive(Debug, Default)]
pub struct CacheInner {
    pub(crate) config: CacheConfig,
    pub(crate) rooms: HashMap<RoomId, CachedRoom>,
    pub(crate) channels: HashMap<ChannelId, CachedChannel>,
    pub(crate) users: HashMap<UserId, CachedUser>,
    // TODO: use LruCache and/or Dashmap
    // pub(crate) users: lru::LruCache<UserId, CachedUser>,
}

#[derive(Debug, Clone)]
pub struct CachedRoom {
    pub inner: Room,
    pub members: HashMap<UserId, RoomMember>,
    pub channels: HashMap<ChannelId, CachedChannel>, // contains threads
    pub roles: HashMap<RoleId, Role>,
    pub(crate) perm_roles: HashMap<RoleId, (PermSet, u16)>,
}

// TODO: impl Deref for CachedFoo structs?

#[derive(Debug, Clone)]
pub struct CachedUser {
    pub inner: User,

    /// your relationship with this user, if it is known
    pub relationship: Option<Relationship>,
    // TODO: use this instead of inner.presence?
    // pub presence: Option<Presence>,
}

#[derive(Debug, Clone)]
pub struct CachedCurrentUser {
    pub inner: User,
    // TODO: use this instead of inner.presence?
    // pub presence: Option<Presence>,
}

#[derive(Debug, Clone)]
pub struct CachedChannel {
    pub inner: Channel,
    pub members: HashMap<UserId, ThreadMember>,
    // PERF: don't use Arc<RwLock<_>>? what do i use instead?
    pub(crate) messages: Arc<RwLock<MessagesInner>>,
    pub(crate) perm_roles: HashMap<RoleId, PermSet>,
    pub(crate) perm_users: HashMap<UserId, PermSet>,
}

#[derive(Debug, Clone)]
pub(crate) struct PermSet {
    pub(crate) allow: PermissionBits,
    pub(crate) deny: PermissionBits,
}

impl From<&Role> for PermSet {
    fn from(value: &Role) -> Self {
        Self {
            allow: value.allow.as_slice().into(),
            deny: value.deny.as_slice().into(),
        }
    }
}

impl From<&PermissionOverwrite> for PermSet {
    fn from(value: &PermissionOverwrite) -> Self {
        Self {
            allow: value.allow.as_slice().into(),
            deny: value.deny.as_slice().into(),
        }
    }
}

pub struct CacheRef<'a, V> {
    inner: &'a V,
}

/// something that can be identified with an id
pub trait Identifiable {
    type Id;

    fn id(&self) -> Self::Id;
}

// TODO: allow using custom models for cache
// pub trait CacheableChannel: From<Channel> {
//     fn id(&self) -> ChannelId;
//     fn parent_id(&self) -> Option<ChannelId>;
//     fn room_id(&self) -> Option<RoomId>;
//     fn channel_type(&self) -> ChanneType;
//     // permission overwrites, last message id, last pin timestamp?
// }

impl Identifiable for Channel {
    type Id = ChannelId;

    fn id(&self) -> Self::Id {
        self.id
    }
}
// TODO: impl Identifiable for room, user

impl<'a, V> std::ops::Deref for CacheRef<'a, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

pub type CachedRoomRef<'a> = CacheRef<'a, CachedRoom>;
pub type CachedUserRef<'a> = CacheRef<'a, CachedUser>;
pub type CachedCurrentUserRef<'a> = CacheRef<'a, CachedCurrentUser>;
pub type CachedChannelRef<'a> = CacheRef<'a, CachedChannel>;

#[derive(Debug)]
pub struct CacheStats {
    pub rooms: usize,
    pub channels: usize,
    pub users: usize,
    // TODO: messages, emojis, room members, presences, per-room stats, per-channel stats
}

// TODO: cache members, roles, emoji
impl Cache {
    pub fn new() -> Cache {
        Self::default()
    }

    pub fn builder() -> CacheBuilder {
        CacheBuilder::default()
    }

    /// get cache config
    pub fn config(&self) -> &CacheConfig {
        &self.inner.config
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

    /// get a reference to the current user
    pub fn current_user(&self) -> Option<CachedCurrentUserRef<'_>> {
        todo!()
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
