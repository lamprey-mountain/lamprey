use std::sync::Arc;

use common::v1::types::{ChannelId, MemberListGroupId, RoleId, RoomId, UserId};

use crate::services::member_lists::visibility::MemberListVisibility;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberListTarget {
    Room(RoomId),
    Channel(ChannelId),
}

/// A member list identifier from the API
// TODO; better name for this
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberListKey1 {
    Room(RoomId),
    RoomChannel(RoomId, ChannelId),
    DmChannel(ChannelId),
}

/// A deduplicated member list key for the server
///
/// Used to deduplicate member lists with identical members
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum MemberListKey {
    /// The entire member list of a room
    Room(RoomId),

    /// A channel in a room
    RoomChannel(RoomId, MemberListVisibility),

    /// A thread in a room's channel
    RoomThread(RoomId, MemberListVisibility, ChannelId),

    /// A DM channel
    Dm(ChannelId),
}

/// Member group classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemberGroupInfo {
    Hoisted { role_position: u64, role_id: RoleId },
    Online,
    Offline,
}

#[derive(Debug)]
/// Member group data with users
pub struct MemberListGroupData {
    pub info: MemberGroupInfo,
    pub users: Vec<UserId>,
}

/// Unique key for sorting members
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberKey {
    pub group: MemberGroupInfo,
    /// either the override_name or user name
    pub name: Arc<str>,
    pub user_id: UserId,
}

impl MemberListKey1 {
    /// Get the room ID if applicable
    pub fn room_id(&self) -> Option<RoomId> {
        match self {
            Self::Room(id) => Some(*id),
            Self::RoomChannel(id, _) => Some(*id),
            Self::DmChannel(_) => None,
        }
    }

    /// Get the channel ID if applicable
    pub fn channel_id(&self) -> Option<ChannelId> {
        match self {
            Self::Room(_) => None,
            Self::RoomChannel(_, id) => Some(*id),
            Self::DmChannel(id) => Some(*id),
        }
    }
}

impl PartialOrd for MemberKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MemberKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.group.cmp(&other.group) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        match self.name.cmp(&other.name) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        self.user_id.cmp(&other.user_id)
    }
}

impl From<MemberGroupInfo> for MemberListGroupId {
    fn from(value: MemberGroupInfo) -> Self {
        match value {
            MemberGroupInfo::Hoisted { role_id, .. } => MemberListGroupId::Role(role_id),
            MemberGroupInfo::Online => MemberListGroupId::Online,
            MemberGroupInfo::Offline => MemberListGroupId::Offline,
        }
    }
}
