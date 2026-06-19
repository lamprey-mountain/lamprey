#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{ChannelId, RoleId, RoomId, RoomMember, ThreadMember, User, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncSubscribeMemberList {
    pub room_id: Option<RoomId>,

    // renamed from thread_id
    pub channel_id: Option<ChannelId>,

    /// the ranges to subscribe to
    pub ranges: Vec<(u64, u64)>,
}

// TODO: skip sending room_members/thread_members/users if the client already has them
// NOTE: maybe i should move users/room_members/thread_members to the MemberListSync event
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MemberListGroup {
    pub id: MemberListGroupId,
    pub count: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MemberListGroupId {
    /// online members without a hoisted role
    Online,

    /// offline members, including those with a role
    Offline,

    /// hoisted roles
    #[cfg_attr(feature = "serde", serde(untagged))]
    Role(RoleId),
}
