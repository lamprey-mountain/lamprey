#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::util::{Diff, Time};
use crate::v1::types::{RoomMember, User, UserId};

use super::ChannelId;

// NOTE: maybe i could merge the room_member and thread_member types?

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadMember {
    pub thread_id: ChannelId,
    pub user_id: UserId,

    // TODO: remove entirely
    pub membership: ThreadMembership,

    /// When this member joined the thread
    pub joined_at: Time,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ThreadMemberPut {
    // remove?
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ThreadMemberPatch {
    // remove?
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ThreadMembership {
    /// joined. a member of this thread.
    Join,

    /// kicked or left, can rejoin with an invite
    /// todo: can still view messages up until then
    Leave,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams, ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ChannelMemberSearch {
    pub query: String,

    #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
    pub limit: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
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

impl Diff<ThreadMember> for ThreadMemberPatch {
    fn changes(&self, _other: &ThreadMember) -> bool {
        false
    }
}
