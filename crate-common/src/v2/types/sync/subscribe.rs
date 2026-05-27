#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    document::DocumentStateVector, ChannelId, ConnectionId, DocumentBranchId, InviteCode, RedexId,
    RoleId, RoomId, RoomMember, ThreadMember, User, UserId,
};

/// update what the client is subscribed to
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum SyncSubscriptionsUpdate {
    /// replace subscriptions
    Replace(SyncSubscriptionsReplace),

    /// subscribe to a document
    SubscribeDocument {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        state_vector: Option<Box<DocumentStateVector>>,
    },

    UnsubscribeDocument {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
    },
}

/// replace a client's subscriptions
///
/// leaving a field as None will skip updating. set it to an empty vec to clear.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct SyncSubscriptionsReplace {
    /// the member lists to subscribe to
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub member_lists: Option<Vec<SyncSubscribeMemberList>>,

    /// the scripts to subscribe to
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub scripts: Option<Vec<SyncSubscribeScript>>,

    /// the user profiles to subscribe to
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    pub users: Option<Vec<UserId>>,

    /// the invite to subscribe to
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    pub invites: Option<Vec<InviteCode>>,

    /// the rooms to subscribe to (lurking)
    #[cfg_attr(feature = "utoipa", schema(required = false, max_length = 8))]
    #[cfg_attr(feature = "validator", validate(length(max = 8)))]
    pub rooms: Option<Vec<RoomId>>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncSubscriptionsState {
    pub member_lists: Vec<SyncSubscribeMemberList>,
    pub documents: Vec<SyncSubscriptionsStateDocument>,
    pub scripts: Vec<SyncSubscribeScript>,
    pub users: Vec<UserId>,
    pub invites: Vec<InviteCode>,
    pub rooms: Vec<RoomId>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncSubscriptionsStateDocument {
    pub channel_id: ChannelId,
    pub branch_id: DocumentBranchId,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DispatchSubscriptions {
    pub connection_id: ConnectionId,

    #[serde(flatten)]
    pub inner: DispatchSubscriptionsInner,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(tag = "type"))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum DispatchSubscriptionsInner {
    /// these are your current subscriptions
    Subscriptions { state: SyncSubscriptionsState },

    /// confirmation that the client is now subscribed to a document.
    ///
    /// sent after the initial `DocumentEdit` containing the current document
    /// state has been sent. clients should wait for this event before sending
    /// `DocumentPresence` or `DocumentEdit` messages to avoid "not subscribed" errors.
    DocumentSubscribed {
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
    },

    /// an update to a member list
    MemberListDispatch(MemberListDispatch),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncSubscribeMemberList {
    /// the list to subscribe to
    pub target: MemberListTarget,

    /// the ranges to subscribe to
    pub ranges: Vec<MemberListRange>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncSubscribeScript {
    pub channel_id: ChannelId,
    pub redex_id: RedexId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(tag = "type", rename_all = "lowercase")
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MemberListTarget {
    /// subscribe to a room's member list
    Room { room_id: RoomId },

    /// subscribe to a channel's member list
    Channel {
        /// the room id. required if this channel is in a room
        room_id: Option<RoomId>,

        channel_id: ChannelId,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(untagged))]
pub enum MemberListRange {
    /// a static range of items
    Static {
        /// the range of items to subscribe to
        ///
        /// start is inclusive, end is exclusive
        #[cfg_attr(feature = "serde", serde(rename = "static"))]
        static_range: (u64, u64),
    },

    /// a member list group
    Group { group: MemberListGroup },
}

// TODO: skip sending room_members/thread_members/users if the client already has them
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum MemberListOperation {
    /// replace a range of members
    ///
    /// `room_members`, `thread_members`, and `users` may skip users in `items` if the sync worker is sure the user already has that data
    Sync {
        /// the start of the range to replace
        position: u64,

        /// the users in this range
        items: Vec<UserId>,

        /// only returned if channel is in a room and not already cached by client
        // TODO: skip serializing if empty
        room_members: Vec<RoomMember>,

        /// only returned if listing members in a thread and not already cached by client
        // TODO: skip serializing if empty
        thread_members: Vec<ThreadMember>,

        /// users in this range that are not already cached by client
        // TODO: skip serializing if empty
        users: Vec<User>,
    },

    /// insert a member
    Insert {
        position: u64,
        user_id: UserId,

        room_member: Option<Box<RoomMember>>,
        thread_member: Option<Box<ThreadMember>>,
        user: Option<Box<User>>,
    },

    /// delete a range of one or more members
    Delete {
        /// the start of the range to delete
        position: u64,

        /// how many items to delete
        // internally, this will usually will be 1
        count: u64,
    },
}

/// metadata about a group in the member list
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MemberListGroup {
    pub id: MemberListGroupId,
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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MemberListGroupId {
    /// online members
    ///
    /// excludes members with a role
    Online,

    /// offline members
    ///
    /// includes members without a role
    Offline,

    /// members "connected" to this channel
    ///
    /// includes members without a role
    // TODO: currently voice channels and documents use this
    Connected,

    /// hoisted roles
    #[cfg_attr(feature = "serde", serde(untagged))]
    Role(RoleId),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MemberListDispatch {
    /// which user this list sync is for
    pub user_id: UserId,
    pub target: MemberListTarget,
    pub ranges: Vec<MemberListRange>,
    pub ops: Vec<MemberListOperation>,
    pub groups: Vec<MemberListGroup>,
}
