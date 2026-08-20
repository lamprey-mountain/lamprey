use lamprey_macros::record;

use crate::v1::types::{ChannelId, RoleId, RoomId, RoomMember, ThreadMember, User, UserId};

#[record]
#[derive(PartialEq, Eq)]
pub struct SyncSubscribeMemberList {
    pub room_id: Option<RoomId>,

    // renamed from thread_id
    pub channel_id: Option<ChannelId>,

    /// the ranges to subscribe to
    pub ranges: Vec<(u64, u64)>,
}

// TODO: skip sending room_members/thread_members/users if the client already has them
// NOTE: maybe i should move users/room_members/thread_members to the MemberListSync event
#[record]
#[serde(tag = "type")]
pub enum MemberListOp {
    /// replace a range of members
    Sync {
        /// the start of the range
        position: u64,

        /// the users in this range
        items: Vec<UserId>,

        /// only returned if channel is in a room and not already cached by client
        room_members: Option<Vec<RoomMember>>,

        /// only returned if listing members in a thread and not already cached by client
        thread_members: Option<Vec<ThreadMember>>,

        /// users in this range that are not already cached by client
        users: Option<Vec<User>>,
    },

    /// insert a member
    Insert {
        position: u64,
        user_id: UserId,
        room_member: Option<RoomMember>,
        thread_member: Option<ThreadMember>,
        user: Option<Box<User>>,
    },

    /// delete a range of one or more members
    Delete {
        position: u64,
        // usually will be 1
        count: u64,
    },
}

/// information about a group of members
#[record]
pub struct MemberListGroup {
    pub id: MemberListGroupId,
    pub count: u64,
}

/// a unique identifier for a member group
#[record]
#[derive(Copy, PartialEq, Eq)]
pub enum MemberListGroupId {
    /// members connected to the current channel
    ///
    /// only exists for voice channels and documents
    Connected,

    /// online members without a hoisted role
    Online,

    /// offline members, including those with a role
    Offline,

    /// hoisted roles
    #[serde(untagged)]
    Role(RoleId),
}
