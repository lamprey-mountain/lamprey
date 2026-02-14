//! cached/in memory rooms

use std::sync::Arc;

use common::v1::types::{
    Channel, ChannelId, Role, RoleId, Room, RoomMember, RoomSecurity, ThreadMember, User, UserId
};
use dashmap::DashMap;
use tokio::sync::RwLock;

use crate::routes::util::Auth;
use crate::services::cache::permissions::PermissionsCalculator;
use crate::{Error, Result};

pub struct CachedRoom {
    /// the data of the room itself
    pub inner: RwLock<Room>,

    /// every member in this room
    pub members: DashMap<UserId, CachedRoomMember>,

    /// every non-thread channel in this room
    pub channels: DashMap<ChannelId, Channel>,

    /// all roles in the room
    pub roles: DashMap<RoleId, Role>,

    /// all active threads in the room
    pub threads: DashMap<ChannelId, CachedThread>,
}

pub struct CachedRoomMember {
    /// the room member
    pub member: RoomMember,

    /// the user associated with the room member
    pub user: Arc<User>,
}

pub struct CachedThread {
    /// the thread itself
    pub thread: RwLock<Channel>,

    /// thread members
    pub members: DashMap<UserId, ThreadMember>,
    // maybe include first, last message?
}

pub struct CachedRoomSecurity(RoomSecurity);

impl CachedRoom {
    /// update this room's metadata
    pub async fn room_update(&self, room: Room) {
        let mut inner = self.inner.write().await;
        *inner = room;
    }

    // TODO: move more cache updating stuff here (eg. channel_create, channel_update, channel_delete, role_create, role_update, role_delete)

    /// get the permission calculator for this room
    pub async fn permissions(self: Arc<Self>) -> PermissionsCalculator {
        let inner = self.inner.read().await;
        let room_id = inner.id;
        let owner_id = inner.owner_id;
        let public = inner.public;
        drop(inner);
        PermissionsCalculator {
            room_id,
            owner_id,
            public,
            room: Arc::clone(&self),
        }
    }

    pub async fn security(&self) -> CachedRoomSecurity {
        let inner = self.inner.read().await;
        CachedRoomSecurity(inner.security.clone())
    }

    // /// list all channels a user can see
    // pub async fn list_channels_for_user(&self, user_id: UserId) -> Vec<Channel> {
    //     todo!()
    // }
}

impl CachedRoomSecurity {
    pub fn ensure_sudo_if_needed(&self, auth: &Auth) -> Result<()> {
        if self.0.require_sudo {
            auth.ensure_sudo()?;
        }

        Ok(())
    }

    pub fn ensure_mfa_if_needed(&self, auth: &Auth) -> Result<()> {
        if self.0.require_mfa {
            if !auth.user.has_mfa.unwrap_or_default() {
                return Err(Error::BadStatic("mfa required for this action"));
            }
        }

        Ok(())
    }
}
