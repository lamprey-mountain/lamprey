use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{Room, Thread, User, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema), schema(examples("a1b2c3")))]
pub struct InviteCode(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Invite {
    code: InviteCode,
    target: InviteTarget,
    creator_id: UserId,
    // roles: RoleId.array().optional(),
    // expires_at: z.date().optional(),
    // max_uses: Uint.optional(),
    // uses: Uint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum InviteTarget {
    User(User),
    Room(Room),
    Thread(Thread),
}
