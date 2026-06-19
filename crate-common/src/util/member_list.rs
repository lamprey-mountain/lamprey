//! utilities for calculating member lists

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v2::types::{RoleId, UserId, sync::subscribe::MemberListGroupId};

// a sortable version of member list group id
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum MemberGroupKey {
    Connected,
    Hoisted { role_position: u64, role_id: RoleId },
    Online,
    Offline,
}

// sorting key for someone on a member list
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberKey {
    /// the group the member is in
    pub group: MemberGroupKey,

    /// either the room member nickname or global user name
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
        self.group
            .cmp(&other.group)
            .then_with(|| self.name.cmp(&other.name))
            .then_with(|| self.user_id.cmp(&other.user_id))
    }
}

impl From<MemberGroupKey> for MemberListGroupId {
    fn from(value: MemberGroupKey) -> Self {
        match value {
            MemberGroupKey::Connected => MemberListGroupId::Connected,
            MemberGroupKey::Hoisted { role_id, .. } => MemberListGroupId::Role(role_id),
            MemberGroupKey::Online => MemberListGroupId::Online,
            MemberGroupKey::Offline => MemberListGroupId::Offline,
        }
    }
}

#[cfg(any())]
mod algo {
    use std::{
        collections::{BTreeMap, HashMap},
        sync::Arc,
    };

    use crate::{
        util::member_list::{MemberGroupKey, MemberKey},
        v1::types::{MemberListGroup, RoomMember, ThreadMember, User, UserId, presence::Presence},
        v2::types::sync::subscribe::MemberListDispatch,
    };

    // NOTE: do i want to split apart "list for diffing" and "list for patching", or keep both as one struct?
    pub struct List {
        /// ordered map of members for range queries and position tracking
        members: BTreeMap<MemberKey, UserId>,

        /// reverse lookup: UserId -> MemberKey
        // PERF: share between member lists (store in room/roomdata?)
        user_to_key: HashMap<UserId, MemberKey>,

        /// group summaries (id and count)
        groups: BTreeMap<MemberGroupKey, MemberListGroup>,
    }

    // maybe have some sort of list cache type
    // what if i had one big list per room then filtered it per channel based on visibility? unsure how performant it would be, and it wouldn't work with Connection so probably not

    enum ListInput {
        RoomMemberCreate(Arc<RoomMember>, Arc<User>),
        RoomMemberUpdate(Arc<RoomMember>, Arc<User>),
        RoomMemberDelete(UserId),
        ThreadMemberUpsert(Vec<ThreadMember>, Vec<UserId>), // added, removed
        PresenceUpdate(UserId, Arc<Presence>),
        RoleUpdate,
        RoleDelete,
        RoleReorder,
        ChannelUpdate,
    }

    // impl from messagesync for listinput
    // impl from dispatch for listinput

    impl List {
        pub fn diff_v1(&mut self, sync: &crate::v1::types::MessageSync) {
            todo!()
        }

        pub fn diff_v2(&mut self, sync: &crate::v2::types::sync::Dispatch) {
            todo!()
        }

        pub fn diff(&mut self, input: ListInput) {
            todo!()
        }

        pub fn apply(&mut self, dispatch: MemberListDispatch) {
            todo!()
        }
    }

    // fn to get member count, online count (sum groups)

    // TODO: util for integrating dispatches into List
    // TODO: util for differentiating/calculating dispatches for a List
    //
    // see also:
    // crate-backend-services/src/services/rooms/member_lists.rs
    // crate-backend-services/src/services/member_lists.rs
    // crate-backend/src/services/member_lists/mod.rs
    // frontend/src/api/services/MemberListService.ts
}
