#![allow(unused)] // TEMP: suppress warnings here for now

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use common::{
    v1::types::{MemberListGroup, MemberListGroupId, MessageSync, UserId},
    v2::types::{ChannelId, RoleId, RoomId},
};

use crate::{Client, cache::Cache};

/// a list of members, displayed in the sidebar
pub struct MemberList {
    /// cache for users, members
    cache: Arc<dyn Cache>,

    /// ordered map of members for range queries and position tracking
    members: BTreeMap<MemberKey, UserId>,

    /// reverse lookup: UserId -> MemberKey
    user_to_key: HashMap<UserId, MemberKey>,

    /// group summaries (id and count)
    groups: BTreeMap<MemberGroupInfo, MemberListGroup>,
}

// /// shared context between member lists in a room
// struct MemberListRoom {
//     user_to_key: HashMap<UserId, MemberKey>,
// }

pub enum Event {
    /// member list updated
    Update,
}

impl MemberList {
    fn handle_sync(&mut self, msg: MessageSync) {
        let MessageSync::MemberListSync { .. } = msg else {
            return;
        };

        todo!()
    }

    // pub fn target(&self) -> MemberListTarget {
    //     todo!()
    // }
}

impl Client {
    /// get the member list for a room
    pub fn members_room(&self, _room_id: RoomId) -> MemberList {
        todo!()
    }

    /// get the member list for a channel
    pub fn members_channel(&self, _channel_id: ChannelId) -> MemberList {
        todo!()
    }

    // internal helper?
    fn members(&self) -> MemberList {
        todo!()
    }
}

// TODO: move this to crate-common?
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
