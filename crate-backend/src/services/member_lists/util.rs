use std::sync::Arc;

use common::v1::types::{ChannelId, RoleId, RoomId, UserId};

use crate::services::member_lists::visibility::MemberListVisibility;
use crate::Result;

/// a member list identifier from the api
pub enum MemberListKey1 {
    Room(RoomId),
    // could be a thread
    RoomChannel(RoomId, ChannelId),
    DmChannel(ChannelId),
}

/// a deduplicated member list for the server
///
/// used to deduplicate member lists with identical members
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum MemberListKey {
    /// the entire member list of a room
    Room(RoomId),

    /// a channel in a room
    RoomChannel(RoomId, MemberListVisibility),

    /// a thread in a room's channel
    RoomThread(RoomId, MemberListVisibility, ChannelId),

    // // alternative structure
    // RoomChannel {
    //     room_id: RoomId,
    //     visibility: MemberListVisibility,
    //     is_thread: bool,
    //     channel_id: ChannelId,
    // },
    /// a dm channel
    ///
    /// (maybe remove later?)
    Dm(ChannelId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberGroupInfo {
    Online,
    Offline,
    Hoisted(RoleId),
}

#[derive(Debug)]
pub struct MemberListGroupData {
    info: MemberGroupInfo,
    users: Vec<UserId>,
}

// may be removed..? its only useful if i use the btreemap idea
#[derive(Debug, PartialEq, Eq)]
pub struct MemberKey {
    /// role position, -1 used for online, -2 used for offline
    role_pos: i64,

    /// either the override_name or user name
    name: Arc<str>,
}

impl MemberListKey1 {
    pub fn new(room_id: Option<RoomId>, channel_id: Option<ChannelId>) -> Result<Self> {
        todo!()
    }

    pub fn room_id(&self) -> Option<RoomId> {
        todo!()
    }

    pub fn channel_id(&self) -> Option<ChannelId> {
        todo!()
    }
}

impl PartialOrd for MemberKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MemberKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.role_pos.cmp(&other.role_pos) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        self.name.cmp(&other.name)
    }
}
