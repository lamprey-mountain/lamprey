use std::fmt;

use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::util::Time;
use crate::{PaginationKey, RoomId, ThreadId, UserId};

use super::{Room, Thread, User};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[cfg_attr(feature = "utoipa", derive(ToSchema), schema(examples("a1b2c3")))]
pub struct InviteCode(pub String);

// FIXME: uncomment #[deprecated] attrs (disabled to prevent extraneous warnings for now)

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Invite {
    pub code: InviteCode,
    pub target: InviteTarget,

    // #[deprecated = "use creator_id"]
    pub creator: User,
    pub creator_id: UserId,

    pub created_at: Time,

    pub expires_at: Option<Time>,
    // invites that automatically apply a certain role?
    // pub roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct InviteWithMetadata {
    pub max_uses: Option<u64>,
    pub uses: u64,

    #[serde(flatten)]
    pub invite: Invite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum InviteTarget {
    User { user: User },

    Room { room: Room },

    Thread { room: Room, thread: Thread },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum InviteTargetId {
    User {
        user_id: UserId,
    },

    Room {
        room_id: RoomId,
    },

    Thread {
        room_id: RoomId,
        thread_id: ThreadId,
    },
}

// more flexible invite restrictions?
// seems like the wrong way to implement invites...
// enum InviteRestriction {
//     MaxUses(u64),
//     Expires(u64),
//     UserIds(Vec<User>),
// }

impl fmt::Display for InviteCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PaginationKey for InviteCode {
    fn min() -> Self {
        InviteCode("".to_string())
    }

    fn max() -> Self {
        InviteCode("ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ".to_string())
    }
}

impl From<InviteWithMetadata> for Invite {
    fn from(value: InviteWithMetadata) -> Self {
        value.invite
    }
}

impl InviteWithMetadata {
    pub fn strip_metadata(self) -> Invite {
        self.into()
    }
}
