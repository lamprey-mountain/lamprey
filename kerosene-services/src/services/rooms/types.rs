//! cached/in memory rooms

use im::HashMap as ImMap;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

use common::v1::types::{
    Channel, ChannelId, MessageSync, Permission, PermissionOverwriteType, Role, RoleId, Room,
    RoomFeature, RoomMember, ThreadMember, User, UserId,
};
use lamprey_backend_core::types::search::ChannelVisibility;

use crate::compat::routes::util::auth::Auth4 as Auth;
use crate::prelude::*;
use crate::services::cache::PermissionsCalculator;
use crate::services::rooms::utils::sync_room_id;
use crate::types::PermissionBits;

/// a snapshot of a room's state at a point in time.
#[derive(Debug, Clone)]
pub enum RoomSnapshot {
    Available(Arc<LoadedRoom>),
    Unavailable(RoomUnavailable),
}

impl RoomSnapshot {
    pub fn loading() -> Self {
        Self::Unavailable(RoomUnavailable::Loading)
    }

    pub fn not_found() -> Self {
        Self::Unavailable(RoomUnavailable::NotFound)
    }

    pub fn deleted() -> Self {
        Self::Unavailable(RoomUnavailable::Deleted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomUnavailable {
    /// the room is being loaded from the database
    Loading,

    /// the room could not be found
    NotFound,

    /// the room is deleted
    Deleted,

    // /// the room is quarantined
    // Quarantined,

    // /// the federated server the room is on is offline
    // FederationOffline,
    // FederationTimeout,
    // // etc..
    /// too many events were received and the room actor is backlogged
    Backlogged,
}

impl RoomUnavailable {
    // /// whether `.ready()` should always fail when this reason is encountered
    // ///
    // /// otherwise, ready may continue waiting for the room to become available
    // pub fn is_fatal(&self) -> bool {
    //     matches!(self, Self::NotFound | Self::Deleted | Self::Quarantined)
    // }
}

/// a fully loaded room from the database
#[derive(Debug, Clone)]
pub struct LoadedRoom {
    pub room: Arc<Room>,
    pub members: RoomMembers,

    /// all channels in this room
    pub channels: ImMap<ChannelId, CachedChannel>,

    /// all roles in this room
    pub roles: ImMap<RoleId, CachedRole>,

    /// all loaded/active threads in this room
    ///
    /// may be None if threads are still loading
    pub threads: Option<ImMap<ChannelId, CachedThread>>,
    // NOTE: i could move documents, flumes, automod, etc here? note that flumes can exist outside of a room
    // pub documents: Option<ImMap<EditContextId, Document>>,
}

/// the members for a room
#[derive(Debug, Clone)]
pub enum RoomMembers {
    /// all room members are loaded on the local server
    Loaded {
        members: ImMap<UserId, CachedRoomMember>,
        // TODO: add member_lists
        // member_lists: ImMap<ListTarget, List>,
    },

    /// members are currently loading
    Loading,
    // /// this is a server room, so members will only be loaded as needed
    // Server {
    //     members: HashMap<UserId, CachedRoomMember>,
    // },

    // /// currently proxying room member requests to either a remote node or a federated server
    // Federated {
    //     member_lists: HashMap<ListTarget, ProxiedList>,
    // },
}

impl RoomMembers {
    pub fn get(&self, user_id: &UserId) -> Option<&CachedRoomMember> {
        match self {
            Self::Loaded { members } => members.get(user_id),
            Self::Loading => None,
        }
    }

    // TODO: make this more robust against mutations during loading

    pub fn insert(&mut self, user_id: UserId, member: CachedRoomMember) {
        if let Self::Loaded { members } = self {
            members.insert(user_id, member);
        } else {
            warn!("tried to insert() a new member into RoomMembers::Loading");
        }
    }

    pub fn remove(&mut self, user_id: &UserId) -> Option<CachedRoomMember> {
        if let Self::Loaded { members } = self {
            members.remove(user_id)
        } else {
            warn!("tried to remove() a member into RoomMembers::Loading");
            None
        }
    }
}

// TODO: rename to LoadedRoomMember
#[derive(Debug, Clone)]
pub struct CachedRoomMember {
    /// the room member
    pub member: RoomMember,

    /// the user associated with the room member
    pub user: Arc<User>,
}

// TODO: rename to LoadedThread
#[derive(Debug, Clone)]
pub struct CachedThread {
    /// the thread itself
    pub thread: Channel,

    /// thread members
    pub members: ImMap<UserId, ThreadMember>,
}

#[derive(Clone, Debug)]
pub struct CachedChannel {
    /// the channel itself
    pub inner: Channel,

    /// channel permission overwrites as bitfields
    pub overwrites: ImMap<Uuid, CachedPermissionOverwrite>,
}

#[derive(Clone, Debug)]
pub struct CachedRole {
    /// the role itself
    pub inner: Role,

    /// allowed permissions as a bitfield
    pub allow: PermissionBits,

    /// denied permissions as a bitfield
    pub deny: PermissionBits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedPermissionOverwrite {
    /// id of role or user
    pub id: Uuid,

    /// whether this is for a user or role
    pub ty: PermissionOverwriteType,

    /// allowed permissions as a bitfield
    pub allow: PermissionBits,

    /// denied permissions as a bitfield
    pub deny: PermissionBits,
}

impl RoomSnapshot {
    pub fn get_data(&self) -> Option<&Arc<LoadedRoom>> {
        match self {
            Self::Available(data) => Some(data),
            _ => None,
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Available(_))
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::Unavailable(RoomUnavailable::NotFound))
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Unavailable(RoomUnavailable::Loading))
    }

    pub fn is_unavailable(&self) -> bool {
        matches!(self, Self::Unavailable(_))
    }

    pub fn get_member(&self, user_id: &UserId) -> Option<&CachedRoomMember> {
        self.get_data()?.members.get(user_id)
    }

    pub fn get_channel(&self, channel_id: &ChannelId) -> Option<&CachedChannel> {
        self.get_data()?.channels.get(channel_id)
    }

    pub fn get_role(&self, role_id: &RoleId) -> Option<&CachedRole> {
        self.get_data()?.roles.get(role_id)
    }

    pub fn get_roles(&self) -> Option<Vec<Role>> {
        let data = self.get_data()?;
        Some(data.roles.values().map(|r| r.inner.clone()).collect())
    }

    pub fn ensure_sudo_if_needed(&self, auth: &Auth) -> Result<()> {
        if let Some(data) = self.get_data() {
            if data.room.security.require_sudo {
                auth.ensure_sudo()?;
            }

            Ok(())
        } else {
            Err(Error::BadStatic("room not loaded yet"))
        }
    }

    pub fn ensure_mfa_if_needed(&self, auth: &Auth) -> Result<()> {
        if let Some(data) = self.get_data() {
            if data.room.security.require_mfa {
                if !auth
                    .user()
                    .map(|u| u.has_mfa.unwrap_or_default())
                    .unwrap_or_default()
                {
                    return Err(Error::BadStatic("mfa required for this action"));
                }
            }
            Ok(())
        } else {
            Err(Error::BadStatic("room not loaded yet"))
        }
    }

    pub fn ensure_feature(&self, feature: &RoomFeature) -> Result<()> {
        if let Some(data) = self.get_data() {
            if !data.room.features.0.contains(feature) {
                return Err(Error::BadStatic("feature not enabled"));
            }

            Ok(())
        } else {
            Err(Error::BadStatic("room not loaded yet"))
        }
    }

    pub fn channel_visibilities(
        self: Arc<Self>,
        user_id: UserId,
        state: Globals,
    ) -> Vec<ChannelVisibility> {
        let Some(data) = self.get_data() else {
            return vec![];
        };

        let calc = PermissionsCalculator {
            state,
            room_id: data.room.id,
            owner_id: data.room.owner_id,
            public: data.room.public,
            room: Arc::clone(&self),
        };

        data.channels
            .values()
            .filter_map(|chan| {
                let perms = calc
                    .query2(Some(user_id), Some(&chan.inner))
                    .expect("room has data");

                let Ok(perms) = perms.ensure_view() else {
                    return None;
                };

                Some(ChannelVisibility {
                    id: chan.inner.id,
                    can_view_private_threads: perms.has(Permission::ThreadManage),
                })
            })
            .collect()
    }

    /// get a permission calculator for this room
    pub fn permissions(self: &Arc<Self>, state: Globals) -> Option<PermissionsCalculator> {
        let data = self.get_data()?;
        Some(PermissionsCalculator {
            state: state.clone(),
            room_id: data.room.id,
            owner_id: data.room.owner_id,
            public: data.room.public,
            room: Arc::clone(&self),
        })
    }
}

impl LoadedRoom {
    /// apply a sync message to this state and calculate a new state
    // PERF: don't clone on every sync message? eg. return None if an event isnt applicable. or take and return Arc
    pub fn apply(&self, msg: &MessageSync) -> Self {
        let mut new_room = self.clone();

        let Some(room_id) = sync_room_id(msg) else {
            return new_room;
        };

        if room_id != self.room.id {
            return new_room;
        };

        match msg {
            MessageSync::RoomUpdate { room } => {
                new_room.room = Arc::new(room.clone());
            }
            // TODO: handle RoomDelete (might need to be handled in actor.rs?)
            // MessageSync::RoomDelete { .. } => { }
            MessageSync::ChannelCreate { channel } => {
                if channel.is_thread() {
                    if let Some(ref mut threads) = new_room.threads {
                        threads.insert(
                            channel.id,
                            CachedThread {
                                thread: *channel.clone(),
                                members: ImMap::new(),
                            },
                        );
                    }
                } else {
                    new_room
                        .channels
                        .insert(channel.id, (*channel.clone()).into());
                }
            }
            MessageSync::ChannelUpdate { channel } => {
                if channel.is_thread() {
                    if let Some(ref mut threads) = new_room.threads {
                        if channel.is_removed() {
                            threads.remove(&channel.id);
                        } else {
                            threads
                                .entry(channel.id)
                                .and_modify(|t| {
                                    t.thread = *channel.clone();
                                })
                                .or_insert_with(|| CachedThread {
                                    thread: *channel.clone(),
                                    members: ImMap::new(),
                                });
                        }
                    }
                } else if channel.is_removed() {
                    new_room.channels.remove(&channel.id);
                } else {
                    new_room
                        .channels
                        .insert(channel.id, (*channel.clone()).into());
                }
            }
            MessageSync::RoleCreate { role } => {
                let allow = PermissionBits::from(&role.allow);
                let deny = PermissionBits::from(&role.deny);
                new_room.roles.insert(
                    role.id,
                    CachedRole {
                        inner: role.clone(),
                        allow,
                        deny,
                    },
                );
            }
            MessageSync::RoleUpdate { role } => {
                let allow = PermissionBits::from(&role.allow);
                let deny = PermissionBits::from(&role.deny);
                new_room.roles.insert(
                    role.id,
                    CachedRole {
                        inner: role.clone(),
                        allow,
                        deny,
                    },
                );
            }
            MessageSync::RoleDelete { role_id, .. } => {
                new_room.roles.remove(role_id);
                if let RoomMembers::Loaded { ref mut members } = new_room.members {
                    for (user_id, member) in members.clone().into_iter() {
                        if member.member.roles.contains(role_id) {
                            let mut updated_member = member.clone();
                            updated_member.member.roles.retain(|r| r != role_id);
                            members.insert(user_id, updated_member);
                        }
                    }
                }
            }
            MessageSync::RoleReorder { roles, .. } => {
                for item in roles {
                    if let Some(mut role) = new_room.roles.get(&item.role_id).cloned() {
                        role.inner.position = item.position;
                        new_room.roles.insert(item.role_id, role);
                    }
                }
            }
            MessageSync::EmojiCreate { .. } => {
                new_room.room = Arc::new(Room {
                    emoji_count: self.room.emoji_count + 1,
                    ..(*self.room).clone()
                });
            }
            MessageSync::EmojiDelete { .. } => {
                new_room.room = Arc::new(Room {
                    emoji_count: self.room.emoji_count - 1,
                    ..(*self.room).clone()
                });
            }
            MessageSync::RoomMemberCreate { member, user } => {
                if let RoomMembers::Loaded { ref mut members } = new_room.members {
                    members.insert(
                        member.user_id,
                        CachedRoomMember {
                            member: member.clone(),
                            user: Arc::new(user.clone()),
                        },
                    );
                }

                new_room.room = Arc::new(Room {
                    member_count: self.room.member_count + 1,
                    ..(*self.room).clone()
                });
            }
            MessageSync::RoomMemberUpdate { member, user } => {
                if let RoomMembers::Loaded { ref mut members } = new_room.members {
                    members.insert(
                        member.user_id,
                        CachedRoomMember {
                            member: member.clone(),
                            user: Arc::new(user.clone()),
                        },
                    );
                }
            }
            MessageSync::RoomMemberDelete { user_id, .. } => {
                if let RoomMembers::Loaded { ref mut members } = new_room.members {
                    members.remove(user_id);
                }

                new_room.room = Arc::new(Room {
                    member_count: self.room.member_count - 1,
                    ..(*self.room).clone()
                });
            }
            MessageSync::ThreadMemberUpsert {
                thread_id,
                added,
                removed,
                ..
            } => {
                if let Some(ref mut threads) = new_room.threads {
                    for member in added {
                        if let Some(thread) = threads.get_mut(thread_id) {
                            thread.members.insert(member.user_id, member.clone());
                        } else if let Some(cached_channel) = new_room.channels.get(thread_id) {
                            threads.insert(
                                *thread_id,
                                CachedThread {
                                    thread: cached_channel.inner.clone(),
                                    members: ImMap::from_iter([(member.user_id, member.clone())]),
                                },
                            );
                        }
                    }

                    for user_id in removed {
                        if let Some(thread) = threads.get_mut(thread_id) {
                            thread.members.remove(user_id);
                        }
                    }
                }
            }
            _ => {}
        };

        new_room
    }

    // pub fn ensure_sudo_if_needed(&self, auth: &Auth) -> Result<()> {
    // pub fn ensure_mfa_if_needed(&self, auth: &Auth) -> Result<()> {
    // pub fn ensure_feature(&self, feature: &RoomFeature) -> Result<()> {
    // pub fn channel_visibilities()
    // pub fn permissions() -> RoomPermissions {}
}

impl From<Channel> for CachedChannel {
    fn from(channel: Channel) -> Self {
        let mut overwrites = ImMap::new();
        for ow in &channel.permission_overwrites {
            overwrites.insert(
                ow.id,
                CachedPermissionOverwrite {
                    id: ow.id,
                    ty: ow.ty,
                    allow: PermissionBits::from(&ow.allow),
                    deny: PermissionBits::from(&ow.deny),
                },
            );
        }

        CachedChannel {
            overwrites,
            inner: channel,
        }
    }
}
