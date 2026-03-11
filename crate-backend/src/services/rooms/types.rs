//! cached/in memory rooms

use im::HashMap as ImMap;
use std::sync::Arc;

use common::v1::types::{
    Channel, ChannelId, MessageSync, PermissionOverwriteType, Role, RoleId, Room, RoomId,
    RoomMember, ThreadMember, User, UserId,
};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::routes::util::Auth;
use crate::types::PermissionBits;
use crate::{Error, Result};

/// A snapshot of a room's state at a point in time.
/// Used for zero-latency reads.
#[derive(Debug, Clone)]
pub enum RoomSnapshot {
    /// The room is currently being loaded from the database.
    Loading,

    /// The room is fully loaded, including the complete member list.
    Ready(Arc<RoomData>),

    /// The room metadata, roles, and channels are loaded, but the member list is not.
    /// This is used for large rooms or rooms that haven't been "activated" by a member list request.
    WithoutMembers(Arc<RoomData>),

    /// The room was not found in the database.
    // remove this? i dont need an actor for a non-existent room. maybe i should cache "negative stuff" though.
    NotFound,

    /// The room is currently unavailable (e.g. backlogged).
    Unavailable(RoomUnavailable),
}

#[derive(Debug, Clone, Copy)]
pub struct RoomUnavailable {
    pub reason: RoomUnavailableReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomUnavailableReason {
    /// too many events were received and the room actor is backlogged
    // maybe rename to "overloaded"?
    Backlogged,
}

#[derive(Debug, Clone)]
pub struct RoomData {
    pub room: Room,
    pub members: ImMap<UserId, CachedRoomMember>,
    pub channels: ImMap<ChannelId, CachedChannel>,
    pub roles: ImMap<RoleId, CachedRole>,
    pub threads: ImMap<ChannelId, CachedThread>,
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

    /// Ensure that the room members are loaded.
    EnsureMembers,

    /// Close the actor (usually due to idle timeout).
    Close,
}

/// A handle to a room actor.
#[derive(Clone)]
pub struct RoomHandle {
    pub room_id: RoomId,
    pub tx: mpsc::Sender<RoomCommand>,
    pub snapshot_rx: watch::Receiver<Arc<RoomSnapshot>>,
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
    pub thread: Channel,

    /// thread members
    pub members: ImMap<UserId, ThreadMember>,
}

#[derive(Clone, Debug)]
pub struct CachedChannel {
    /// the channel itself
    pub inner: Channel,

    /// channel permission overwrites as bitfields
    // maybe dont make this an ImMap
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
    pub fn get_data(&self) -> Option<&Arc<RoomData>> {
        match self {
            Self::Ready(data) | Self::WithoutMembers(data) => Some(data),
            _ => None,
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound)
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn is_without_members(&self) -> bool {
        matches!(self, Self::WithoutMembers(_))
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

    pub fn ensure_sudo_if_needed(&self, auth: &Auth) -> Result<()> {
        if let Some(data) = self.get_data() {
            if data.room.security.require_sudo {
                auth.ensure_sudo()?;
            }
        }

        Ok(())
    }

    pub fn ensure_mfa_if_needed(&self, auth: &Auth) -> Result<()> {
        if let Some(data) = self.get_data() {
            if data.room.security.require_mfa {
                if !auth.user.has_mfa.unwrap_or_default() {
                    return Err(Error::BadStatic("mfa required for this action"));
                }
            }
        }

        Ok(())
    }
}
