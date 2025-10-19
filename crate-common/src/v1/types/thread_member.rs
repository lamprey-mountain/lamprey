use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::util::{Diff, Time};
use crate::v1::types::UserId;

use super::ChannelId;

// NOTE: maybe i could merge the room_member and thread_member types?

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadMember {
    pub thread_id: ChannelId,
    pub user_id: UserId,

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

impl Diff<ThreadMember> for ThreadMemberPatch {
    fn changes(&self, _other: &ThreadMember) -> bool {
        false
    }
}
