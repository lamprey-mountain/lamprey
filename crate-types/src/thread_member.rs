use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::RoomMember;

use super::ThreadId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadMember {
    pub room_member: RoomMember,
    pub thread_id: ThreadId,
    pub joined_at: time::OffsetDateTime,
    // pub updated_at: time::OffsetDateTime,
    // pub updated_by: UserId,
    // pub membership: ThreadMemberState ,
}

// enum ThreadMemberState {
//     /// joined
//     Join {
//     },

//     /// kicked or left, can still view messages up until then, can rejoin
//     Left {
//         reason: Option<String>,
//     },

//     /// banned, can still view messages up until they were banned
//     Ban {
//         reason: Option<String>,
//     },
// }
