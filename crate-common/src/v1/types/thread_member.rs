use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::util::{some_option, Diff, Time};
use crate::v1::types::UserId;

use super::ThreadId;

// NOTE: maybe i could merge the room_member and thread_member types?

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadMember {
    pub thread_id: ThreadId,
    pub user_id: UserId,

    pub membership: ThreadMembership,

    /// When this member's membership last changed (joined, left, was kicked, or banned).
    #[deprecated]
    pub membership_updated_at: Time,

    /// When this member joined the thread
    pub joined_at: Time,

    /// aka nickname
    // TODO: remove - not very useful, but can be very confusing
    #[deprecated]
    pub override_name: Option<String>,

    /// like nickname but for description/bio/about
    // TODO: remove - not very useful, but can be very confusing
    #[deprecated]
    pub override_description: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ThreadMemberPut {
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 64)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub override_name: Option<String>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub override_description: Option<String>,
    // pub override_avatar: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ThreadMemberPatch {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    #[serde(default, deserialize_with = "some_option")]
    pub override_name: Option<Option<String>>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[serde(default, deserialize_with = "some_option")]
    pub override_description: Option<Option<String>>,
    // #[serde(default, deserialize_with = "some_option")]
    // pub override_avatar: Option<Option<String>>,
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
    fn changes(&self, other: &ThreadMember) -> bool {
        self.override_name.changes(&other.override_name)
            || self
                .override_description
                .changes(&other.override_description)
    }
}
