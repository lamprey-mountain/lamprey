//! cached/in memory rooms

use std::collections::HashMap;
use std::sync::Arc;

use common::v1::types::{
    Channel, ChannelId, MessageSync, PermissionOverwriteType, Role, RoleId, Room, RoomMember,
    RoomSecurity, ThreadMember, User, UserId,
};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::routes::util::Auth;
use crate::types::PermissionBits;
use crate::Result;

/// A snapshot of a room's state at a point in time.
/// Used for zero-latency reads.
#[derive(Debug, Clone)]
pub struct RoomSnapshot {
    pub room: Room,
    pub status: RoomStatus,
    pub members: HashMap<UserId, Arc<CachedRoomMember>>,
    pub channels: HashMap<ChannelId, Arc<CachedChannel>>,
    pub roles: HashMap<RoleId, Arc<CachedRole>>,
    pub threads: HashMap<ChannelId, Arc<CachedThread>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoomStatus {
    #[default]
    Loading,
    Ready,
    NotFound,
}

/// Commands that can be sent to a room actor.
pub enum RoomCommand {
    /// Update the room state from a sync event.
    Sync(MessageSync),

    /// Member list command.
    MemberList(
        crate::services::member_lists::util::MemberListKey,
        crate::services::member_lists::actor::MemberListCommand,
    ),

    /// Subscribe to member list events.
    MemberListSubscribe(
        crate::services::member_lists::util::MemberListKey,
        tokio::sync::broadcast::Sender<crate::services::member_lists::actor::MemberListEvent>,
    ),

    /// Close the actor (usually due to idle timeout).
    Close,
}

/// A handle to a room actor.
#[derive(Clone)]
pub struct RoomHandle {
    pub room_id: crate::types::RoomId,
    pub tx: mpsc::Sender<RoomCommand>,
    pub snapshot: watch::Receiver<Arc<RoomSnapshot>>,
}

#[derive(Debug, Clone)]
pub struct CachedRoomMember {
    /// the room member
    pub member: RoomMember,

    /// the user associated with the room member
    pub user: Arc<User>,
}

#[derive(Debug, Clone)]
pub struct CachedThread {
    /// the thread itself
    pub thread: Arc<Channel>,

    /// thread members
    pub members: HashMap<UserId, ThreadMember>,
}

#[derive(Clone, Debug)]
pub struct CachedChannel {
    /// the channel itself
    pub inner: Channel,

    /// channel permission overwrites as bitfields
    pub overwrites: HashMap<Uuid, CachedPermissionOverwrite>,
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
    pub fn default_with_id(room_id: common::v1::types::RoomId) -> Self {
        Self {
            room: Room {
                id: room_id,
                version_id: Uuid::nil(),
                owner_id: None,
                name: String::new(),
                description: None,
                icon: None,
                banner: None,
                room_type: common::v1::types::RoomType::Default,
                member_count: 0,
                online_count: 0,
                channel_count: 0,
                emoji_count: 0,
                archived_at: None,
                public: false,
                deleted_at: None,
                welcome_channel_id: None,
                quarantined: false,
                preferences: None,
                security: RoomSecurity::default(),
                afk_channel_id: None,
                afk_channel_timeout: 0,
            },
            status: RoomStatus::Loading,
            members: HashMap::new(),
            channels: HashMap::new(),
            roles: HashMap::new(),
            threads: HashMap::new(),
        }
    }

    pub fn get_member(&self, user_id: &UserId) -> Option<&Arc<CachedRoomMember>> {
        self.members.get(user_id)
    }

    pub fn get_channel(&self, channel_id: &ChannelId) -> Option<&Arc<CachedChannel>> {
        self.channels.get(channel_id)
    }

    pub fn get_role(&self, role_id: &RoleId) -> Option<&Arc<CachedRole>> {
        self.roles.get(role_id)
    }

    pub fn ensure_sudo_if_needed(&self, auth: &Auth) -> Result<()> {
        if self.room.security.require_sudo {
            auth.ensure_sudo()?;
        }

        Ok(())
    }

    pub fn ensure_mfa_if_needed(&self, auth: &Auth) -> Result<()> {
        if self.room.security.require_mfa {
            if !auth.user.has_mfa.unwrap_or_default() {
                return Err(crate::Error::BadStatic("mfa required for this action"));
            }
        }

        Ok(())
    }
}
