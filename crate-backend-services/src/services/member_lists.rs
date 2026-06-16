use crate::prelude::*;
use common::{
    v1::types::{MemberListGroup, MemberListGroupId, MemberListOp},
    v2::types::{ChannelId, RoleId, RoomId, UserId},
};
use tokio::sync::mpsc;

pub struct Service {
    //
}

impl Service {
    pub fn new(globals: Globals) -> Self {
        todo!()
    }

    pub fn create_syncer(&self) -> MemberListSyncer {
        todo!()
    }
}

/// manages multiple member lists
///
/// tries to deduplicate data, ie. avoids sending user, room member, and thread member objects the client already has
///
/// handles range filtering
pub struct MemberListSyncer {
    // ...
}

/// a single member list for a room
pub struct MemberList {
    // ...
}

pub struct MemberListQuery {
    pub target: MemberListTarget,
    pub ranges: Vec<(u64, u64)>,
}

pub enum MemberListTarget {
    Room(RoomId),
    Channel(ChannelId),
}

pub enum MemberListSync {
    Sync {
        room_id: Option<RoomId>,
        channel_id: Option<ChannelId>,
        ops: Vec<MemberListOp>,
        groups: Vec<MemberListGroup>,
    },
    // /// initial ranges for a list
    // Initial {},
}

impl MemberListSyncer {
    /// set the queries for this syncer
    pub fn set_queries(&mut self, _queries: &[MemberListQuery]) {
        todo!()
    }

    /// get a ~~stream~~ mpsc receiver for MessageSync events
    pub fn subscribe(&self) -> mpsc::Receiver<Arc<MemberListSync>> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemberGroupInfo {
    // Connected,
    Hoisted { role_position: u64, role_id: RoleId },
    Online,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberKey {
    /// the group the member is in
    pub group: MemberGroupInfo,

    /// either the override_name or user name
    // PERF: smolstr?
    pub name: Box<str>,

    /// tiebreak with user id
    pub user_id: UserId,
}

impl PartialOrd for MemberKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MemberKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // PERF: maybe use then_with
        self.group
            .cmp(&other.group)
            .then(self.name.cmp(&other.name))
            .then(self.user_id.cmp(&other.user_id))
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
