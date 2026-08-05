use lamprey_macros::record;

use crate::v1::types::{
    ChannelId, ConnectionId, DocumentBranchId, InviteCode, RedexId, RoleId, RoomId, RoomMember,
    ThreadMember, User, UserId, document::DocumentStateVector,
};

/// update what the client is subscribed to
#[record]
#[serde(tag = "type")]
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
#[record]
pub struct SyncSubscriptionsReplace {
    /// the member lists to subscribe to
    #[schema(required = false, max_length = 8)]
    #[validate(length(max = 8))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_lists: Option<Vec<SyncSubscribeMemberList>>,

    /// the scripts to subscribe to
    #[schema(required = false, max_length = 8)]
    #[validate(length(max = 8))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<Vec<SyncSubscribeScript>>,

    /// the user profiles to subscribe to
    #[schema(required = false, max_length = 8)]
    #[validate(length(max = 8))]
    pub users: Option<Vec<UserId>>,

    /// the invite to subscribe to
    #[schema(required = false, max_length = 8)]
    #[validate(length(max = 8))]
    pub invites: Option<Vec<InviteCode>>,

    /// the rooms to subscribe to (lurking)
    #[schema(required = false, max_length = 8)]
    #[validate(length(max = 8))]
    pub rooms: Option<Vec<RoomId>>,
}

#[record]
pub struct SyncSubscriptionsState {
    pub member_lists: Vec<SyncSubscribeMemberList>,
    pub documents: Vec<SyncSubscriptionsStateDocument>,
    pub scripts: Vec<SyncSubscribeScript>,
    pub users: Vec<UserId>,
    pub invites: Vec<InviteCode>,
    pub rooms: Vec<RoomId>,
}

#[record]
pub struct SyncSubscriptionsStateDocument {
    pub channel_id: ChannelId,
    pub branch_id: DocumentBranchId,
}

#[record]
pub struct DispatchSubscriptions {
    pub connection_id: ConnectionId,

    #[serde(flatten)]
    pub inner: DispatchSubscriptionsInner,
}

#[record]
#[serde(tag = "type")]
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

#[record]
pub struct SyncSubscribeMemberList {
    /// the list to subscribe to
    pub target: MemberListTarget,

    /// the ranges to subscribe to
    pub ranges: Vec<MemberListRange>,
}

#[record]
pub struct SyncSubscribeScript {
    pub channel_id: ChannelId,
    pub redex_id: RedexId,
}

#[record]
#[derive(PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MemberListTarget {
    /// subscribe to a room's member list
    Room { room_id: RoomId },

    /// subscribe to a channel's member list
    Channel {
        /// the room id. required if this channel is in a room
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
    Static {
        /// the range of items to subscribe to
        ///
        /// start is inclusive, end is exclusive
        #[serde(rename = "static")]
        static_range: (u64, u64),
    },

    /// a member list group
    // TODO: implement
    Group { group: MemberListGroup },
}

// TODO: skip sending room_members/thread_members/users if the client already has them
#[record]
#[serde(tag = "type")]
pub enum MemberListOperation {
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
    // TODO: voice channels and documents will use this
    Connected,

    /// hoisted roles
    #[serde(untagged)]
    Role(RoleId),
}

/// an update to a member list
#[record]
pub struct MemberListDispatch {
    /// which user this sync is for
    pub user_id: UserId,

    /// which member list this sync is for
    pub target: MemberListTarget,

    /// what ranges of the member list are being synced
    pub ranges: Vec<MemberListRange>,

    /// operations to apply to your local copy of the member list
    pub ops: Vec<MemberListOperation>,

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
