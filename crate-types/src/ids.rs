use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::PaginationKey;

#[cfg(not(feature = "utoipa"))]
pub trait Identifier:
    From<Uuid> + Into<Uuid> + Display + Clone + Copy + PartialEq + Eq + PartialOrd + Ord
{
}

#[cfg(feature = "utoipa")]
pub trait Identifier:
    From<Uuid> + Into<Uuid> + Display + Clone + Copy + PartialEq + Eq + PartialOrd + Ord + ToSchema
{
}

impl<T: Identifier> PaginationKey for T {
    fn min() -> Self {
        Uuid::nil().into()
    }

    fn max() -> Self {
        Uuid::max().into()
    }
}

macro_rules! genid {
    ($name:ident, $example:expr) => {
        #[derive(
            Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[cfg_attr(feature = "utoipa", derive(ToSchema), schema(examples($example)))]
        pub struct $name(Uuid);

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
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn into_inner(self) -> Uuid {
                self.into()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl FromStr for $name {
            type Err = <Uuid as FromStr>::Err;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.parse()?))
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
genid!(UserVerId, "00000000-0000-0000-0ver-00000000user");
genid!(RoleId, "00000000-0000-0000-0000-00000000role");
genid!(RoleVerId, "00000000-0000-0000-0ver-00000000role");
genid!(MediaId, "00000000-0000-0000-0000-0000000media");
genid!(SessionId, "00000000-0000-0000-0000-00000session");
// genid!(SessionVerId, "00000000-0000-0000-0ver-00000session");
genid!(AuditLogId, "00000000-0000-0000-0000-0auditlogent");
