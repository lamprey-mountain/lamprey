use std::fmt::Display;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait Identifier:
    From<Uuid> + Into<Uuid> + Display + Clone + Copy + PartialEq + Eq + PartialOrd + Ord
{
}

macro_rules! genid {
    ($name:ident, $example:expr) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            Hash,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            ToSchema,
            Serialize,
            Deserialize,
            sqlx::Type,
        )]
        #[schema(examples($example))]
        #[sqlx(transparent)]
        pub struct $name(pub Uuid);

        impl Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Uuid {
            fn from(val: $name) -> Self {
                val.0
            }
        }

        impl $name {
            pub fn into_inner(self) -> Uuid {
                self.into()
            }
        }

        impl Identifier for $name {}
    };
}

genid!(RoomId, "00000000-0000-0000-0000-00000000room");
genid!(RoomVerId, "00000000-0000-0000-0ver-00000000room");
genid!(ThreadId, "00000000-0000-0000-0000-000000thread");
genid!(ThreadVerId, "00000000-0000-0000-0ver-000000thread");
genid!(MessageId, "00000000-0000-0000-0000-00000message");
genid!(MessageVerId, "00000000-0000-0000-0ver-00000message");
genid!(UserId, "00000000-0000-0000-0000-00000000user");
genid!(RoleId, "00000000-0000-0000-0000-00000000role");
genid!(MediaId, "00000000-0000-0000-0000-0000000media");
genid!(SessionId, "00000000-0000-0000-0000-00000session");
// genid!(AuditLogEntryId, "00000000-0000-0000-0000-0auditlogent");
