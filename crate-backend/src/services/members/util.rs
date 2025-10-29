use std::sync::Arc;

use common::v1::types::{ChannelId, MemberListGroup, RoleId, RoomId, UserId};
use tokio::sync::Notify;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberListTarget {
    Room(RoomId),
    Channel(ChannelId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberGroup {
    Hoisted { role_id: RoleId, role_position: u64 },
    Online,
    Offline,
}

/// a list of members
#[derive(Clone)]
pub struct MemberList {
    pub room_id: Option<RoomId>,
    pub sorted_members: Vec<MemberListItem>,
    pub groups: Vec<MemberListGroup>,
    pub notifier: Arc<Notify>,
}

/// a single member in a member list
#[derive(Clone, PartialEq, Eq)]
pub struct MemberListItem {
    pub user_id: UserId,

    /// the room member override_name, or the user name if it doesnt exist
    pub display_name: Arc<str>,
}

impl PartialOrd for MemberGroup {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MemberGroup {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (self, other) {
            // hoisted roles are ordered by position (lower position = higher precedence = Less)
            (
                MemberGroup::Hoisted {
                    role_position: a, ..
                },
                MemberGroup::Hoisted {
                    role_position: b, ..
                },
            ) => a.cmp(b),

            // hoisted roles come before online and offline
            (MemberGroup::Hoisted { .. }, MemberGroup::Online) => Ordering::Less,
            (MemberGroup::Hoisted { .. }, MemberGroup::Offline) => Ordering::Less,

            // Online comes before offline
            (MemberGroup::Online, MemberGroup::Hoisted { .. }) => Ordering::Greater,
            (MemberGroup::Online, MemberGroup::Online) => Ordering::Equal,
            (MemberGroup::Online, MemberGroup::Offline) => Ordering::Less,

            // Offline comes after everything else
            (MemberGroup::Offline, MemberGroup::Hoisted { .. }) => Ordering::Greater,
            (MemberGroup::Offline, MemberGroup::Online) => Ordering::Greater,
            (MemberGroup::Offline, MemberGroup::Offline) => Ordering::Equal,
        }
    }
}

impl MemberList {
    pub fn new(
        room_id: Option<RoomId>,
        sorted_members: Vec<MemberListItem>,
        groups: Vec<MemberListGroup>,
    ) -> Self {
        Self {
            room_id,
            sorted_members,
            groups,
            notifier: Arc::new(Notify::new()),
        }
    }
}
