use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{Room, Thread, User, UserId};

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
#[schema(examples("a1b2c3"))]
pub struct InviteCode(String);

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct Invite {
    code: InviteCode,
    target: InviteTarget,
    creator_id: UserId,
    // roles: RoleId.array().optional(),
    // expires_at: z.date().optional(),
    // max_uses: Uint.optional(),
    // uses: Uint,
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub enum InviteTarget {
    User(User),
    Room(Room),
    Thread(Thread),
}
