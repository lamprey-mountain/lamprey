//! Client-side caching of api resources

use std::sync::Arc;

use arc_swap::ArcSwap;
use common::{
    v1::types::{
        Channel, PermissionBits, PermissionOverwrite, PermissionOverwriteType, Role, Room,
        RoomMember, ThreadMember, User, presence::Presence, voice::VoiceState,
    },
    v2::types::{ChannelId, RoleId, RoomId, SessionId, UserId},
};
use dashmap::DashMap;

mod config;
mod permissions;
mod update;

pub use config::{CacheBuilder, CacheConfig};
pub use permissions::RoomPermissions;

use crate::messages::MessagesHandle;

/// Caches lamprey api data in memory.
#[derive(Debug, Default, Clone)]
pub struct Cache {
    pub(crate) inner: Arc<CacheInner>,
}

#[derive(Debug, Default)]
pub struct CacheInner {
    pub(crate) config: CacheConfig,
    pub(crate) current: ArcSwap<CacheCurrent>,
    pub(crate) rooms: DashMap<RoomId, Arc<CachedRoom>>,
    pub(crate) channels: DashMap<ChannelId, Arc<CachedChannel>>,
    pub(crate) users: DashMap<UserId, Arc<CachedUser>>,
    // TODO: maybe use LruCache?
    // pub(crate) users: lru::LruCache<UserId, CachedUser>,
}

/// info about the current client
#[derive(Debug, Clone, Copy, Default)]
pub struct CacheCurrent {
    /// the session that the client is authenticating as
    pub session_id: Option<SessionId>,

    /// the user that the client is authenticating as
    pub user_id: Option<UserId>,
}

#[derive(Debug)]
pub struct CachedRoom {
    pub inner: ArcSwap<Room>,
    pub members: DashMap<UserId, RoomMember>,
    pub channels: DashMap<ChannelId, Arc<CachedChannel>>,
    pub threads: DashMap<ChannelId, Arc<CachedChannel>>,
    pub roles: DashMap<RoleId, Role>,
    perm_roles: DashMap<RoleId, (PermSet, u16)>,
}

// TODO: impl Deref for CachedFoo structs?

#[derive(Debug)]
pub struct CachedUser {
    pub inner: ArcSwap<User>,
    // TODO: store relationships here or separately?
    // /// your relationship with this user, if it is known
    // pub relationship: Option<Relationship>,
    /// the user's presence
    // TODO: use this instead of inner.presence?
    pub presence: ArcSwap<Presence>,
}

#[derive(Debug)]
pub struct CachedChannel {
    pub inner: ArcSwap<Channel>,
    pub members: DashMap<UserId, ThreadMember>,
    pub voice_states: DashMap<UserId, Arc<VoiceState>>,
    messages: MessagesHandle,
    perm_roles: DashMap<RoleId, PermSet>,
    perm_users: DashMap<UserId, PermSet>,
}

impl CachedChannel {
    /// wrap an existing channel object
    pub fn new(channel: Channel) -> Self {
        let perm_roles: DashMap<RoleId, PermSet> = DashMap::new();
        let perm_users: DashMap<UserId, PermSet> = DashMap::new();
        for ow in &channel.permission_overwrites {
            let perm = PermSet::from(ow);
            match ow.ty {
                PermissionOverwriteType::Role => {
                    perm_roles.insert(ow.id.into(), perm);
                }
                PermissionOverwriteType::User => {
                    perm_users.insert(ow.id.into(), perm);
                }
            }
        }

        Self {
            inner: ArcSwap::from_pointee(channel),
            members: DashMap::new(),
            voice_states: DashMap::new(),
            messages: MessagesHandle::new(),
            perm_roles,
            perm_users,
        }
    }
}

impl CachedRoom {
    fn new(room: Room) -> Self {
        Self {
            inner: ArcSwap::from_pointee(room),
            members: DashMap::new(),
            channels: DashMap::new(),
            threads: DashMap::new(),
            roles: DashMap::new(),
            perm_roles: DashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct PermSet {
    allow: PermissionBits,
    deny: PermissionBits,
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

// TODO: impl From<User> for CachedUser

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
    pub fn room(&self, id: RoomId) -> Option<Arc<CachedRoom>> {
        self.inner.rooms.get(&id).map(|r| r.value().clone())
    }

    /// get a reference to a channel from its id
    pub fn channel(&self, id: ChannelId) -> Option<Arc<CachedChannel>> {
        self.inner.channels.get(&id).map(|r| r.value().clone())
    }

    /// get a reference to a user from its id
    pub fn user(&self, id: UserId) -> Option<Arc<CachedUser>> {
        self.inner.users.get(&id).map(|r| r.value().clone())
    }

    /// get a reference to the current user
    pub fn current_user(&self) -> Option<Arc<CachedUser>> {
        if let Some(uid) = self.inner.current.load().user_id {
            self.user(uid)
        } else {
            None
        }
    }

    /// get the id of the user the client is authenticating as
    pub fn current_user_id(&self) -> Option<UserId> {
        self.inner.current.load().user_id
    }

    /// get the id of the session the client is authenticating as
    pub fn current_session_id(&self) -> Option<SessionId> {
        self.inner.current.load().session_id
    }

    /// get the voice states of a user across all channels
    pub fn user_voice_states(&self, user_id: UserId) -> impl Iterator<Item = Arc<VoiceState>> + '_ {
        self.inner.channels.iter().filter_map(move |entry| {
            entry
                .voice_states
                .get(&user_id)
                .map(|state| state.value().clone())
        })
    }

    /// iterate over all cached rooms
    pub fn rooms(&self) -> impl Iterator<Item = RoomId> {
        self.inner.rooms.iter().map(|e| *e.key())
    }

    /// iterate over all cached channels
    pub fn channels(&self) -> impl Iterator<Item = ChannelId> {
        self.inner.channels.iter().map(|e| *e.key())
    }

    /// iterate over all cached users
    pub fn users(&self) -> impl Iterator<Item = UserId> {
        self.inner.users.iter().map(|e| *e.key())
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
