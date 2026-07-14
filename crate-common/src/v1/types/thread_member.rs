use lamprey_macros::record;

use crate::v1::types::util::{Diff, Time};
use crate::v1::types::{RoomMember, User, UserId};

use super::ChannelId;

#[record]
#[derive(PartialEq, Eq)]
pub struct ThreadMember {
    pub thread_id: ChannelId,
    pub user_id: UserId,

    /// When this member joined the thread
    pub joined_at: Time,
}

#[record]
#[derive(PartialEq, Eq)]
pub struct ThreadMemberMinimal {
    pub user_id: UserId,

    /// When this member joined the thread
    pub joined_at: Time,
}

impl From<ThreadMember> for ThreadMemberMinimal {
    fn from(value: ThreadMember) -> Self {
        Self {
            user_id: value.user_id,
            joined_at: value.joined_at,
        }
    }
}

#[record]
#[derive(Default, PartialEq, Eq)]
pub struct ThreadMemberPut {
    // remove?
}

#[record]
#[derive(PartialEq, Eq, Diff)]
pub struct ThreadMemberPatch {
    // remove?
}

#[record]
#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(::utoipa::IntoParams))]
pub struct ChannelMemberSearch {
    pub query: String,

    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    pub limit: Option<u16>,
}

#[record]
pub struct ChannelMemberSearchResponse {
    /// the resulting users
    pub users: Vec<User>,

    /// a room member for each returned user
    pub room_members: Vec<RoomMember>,

    /// a thread member for each returned user
    ///
    /// will only be populated if this is a thread
    pub thread_members: Vec<ThreadMember>,
}
