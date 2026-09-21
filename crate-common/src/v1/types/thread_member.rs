use lamprey_macros::record;

use crate::v1::types::util::{Diff, Time};
use crate::v1::types::{ChannelId, RoomMember, User, UserId};

#[record]
pub struct ThreadMember {
    pub thread_id: ChannelId,
    pub user_id: UserId,

    /// When this member joined the thread
    pub joined_at: Time,
}

#[record]
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
#[derive(Default)]
pub struct ThreadMemberCreate {
    // TODO: remove?
}

#[record]
#[derive(Default, Diff)]
#[diff(target = "ThreadMember")]
pub struct ThreadMemberUpdate {
    // TODO: remove?
}

#[record]
#[cfg_attr(feature = "utoipa", derive(::utoipa::IntoParams))]
pub struct ChannelMemberSearch {
    pub query: String,

    #[validate(range(min = 1, max = 100))]
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
