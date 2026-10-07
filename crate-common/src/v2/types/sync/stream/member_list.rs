//! a subscription to a member list

use lamprey_macros::record;

use crate::{
    v1::types::{RoomMember, ThreadMember, User},
    v2::types::{ChannelId, RoleId, RoomId, UserId, sync::stream::StreamProtocol},
};

pub struct Protocol;

#[record]
pub struct Initial {
    /// the list to subscribe to
    pub target: MemberListTarget,

    /// the ranges to subscribe to
    pub ranges: Vec<MemberListRange>,
}

#[record]
#[derive(PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MemberListTarget {
    /// subscribe to a room's member list
    Room { room_id: RoomId },

    /// subscribe to a channel's member list
    ///
    /// this includes thread channels
    Channel {
        /// the id of the room the channel is in
        ///
        /// this is required if the target channel is in a room
        // NOTE: maybe i should make it optional?
        // if i do, maybe i can populate it in the server response...
        room_id: Option<RoomId>,

        channel_id: ChannelId,
    },
}

#[record]
#[serde(untagged)]
pub enum MemberListRange {
    /// a static range of items
    ///
    /// start is inclusive, end is exclusive
    Static(u64, u64),

    #[cfg(any())]
    /// a member list group
    // TODO: implement this
    Group { group: MemberListGroup },
}

#[record]
pub enum Command {
    // /// update subscribed ranges
    // Range(...),
}

#[record]
pub struct Event {
    /// operations to apply to your local copy of the member list
    pub ops: Vec<Operation>,

    /// all groups in this member list
    pub groups: Vec<MemberListGroup>,

    /// relevant room members. the server shouldn't send room members the client already has.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub room_members: Vec<RoomMember>,

    /// relevant thread members. the server shouldn't send thread members the client already has.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thread_members: Vec<ThreadMember>,

    /// relevant users. the server shouldn't send users the client already has.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<User>,
}

// TODO: skip sending room_members/thread_members/users if the client already has them
#[record]
#[serde(tag = "type")]
pub enum Operation {
    /// replace a range of members
    ///
    /// `room_members`, `thread_members`, and `users` may skip users in `items` if the sync worker is sure the user already has that data
    Sync {
        /// the start of the range to replace
        position: u64,

        /// the users in this range
        items: Vec<UserId>,
    },

    /// insert a member
    Insert { position: u64, user_id: UserId },

    /// delete a range of one or more members
    Delete {
        /// the start of the range to delete
        position: u64,

        /// how many items to delete
        // internally, this will usually will be 1
        // NOTE: maybe this should be a NonZeroWhatever?
        count: u64,
    },
}

/// metadata about a group in the member list
#[record]
pub struct MemberListGroup {
    pub id: MemberListGroupId,

    /// the number of users in this group
    pub count: u64,
}

/// identifier for a group in the member list
///
/// ## ordering
///
/// - connected
/// - role (by position)
/// - online
/// - offline
#[record]
#[derive(Copy, PartialEq, Eq)]
pub enum MemberListGroupId {
    /// members connected to the current channel
    ///
    /// only exists for voice channels and documents. includes members without a role
    // TODO: use this in voice channels and documents
    Connected,

    /// online members
    ///
    /// excludes members with a hoisted role
    Online,

    /// offline members
    ///
    /// includes members without a hoisted role
    Offline,

    /// hoisted roles
    #[serde(untagged)]
    Role(RoleId),
}

impl StreamProtocol for Protocol {
    type Initial = Initial;
    type Command = Command;
    type Event = Event;
}
