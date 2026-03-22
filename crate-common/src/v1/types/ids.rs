use core::fmt;
use lamprey_macros::{role_id, room_id, session_id, user_id};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::str::FromStr;
use std::{fmt::Display, marker::PhantomData};
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{util::Time, PaginationKey};

#[cfg(not(feature = "utoipa"))]
pub trait Identifier: From<Uuid> + Into<Uuid> + Display + Clone + Copy {}

#[cfg(feature = "utoipa")]
pub trait Identifier: From<Uuid> + Into<Uuid> + Display + Clone + Copy + ToSchema {}

impl<T: Identifier + Ord + Eq> PaginationKey for T {
    fn min() -> Self {
        Uuid::nil().into()
    }

    fn max() -> Self {
        Uuid::max().into()
    }
}

mod private {
    pub trait Sealed {}
}

pub trait Marker: private::Sealed {
    fn name() -> &'static str;
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id<M: Marker> {
    inner: Uuid,
    phantom: PhantomData<M>,
}

#[cfg(feature = "serde")]
impl<M: Marker> Serialize for Id<M> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Force the UUID to always be serialized as a string.
        // This isn't as efficient as sending the uuid as bytes, but msgpackr keeps trying to deserialize it as a Uint8Array
        // TODO: find out how to make msgpackr work nicely with binary uuids
        serializer.serialize_str(&self.inner.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de, M: Marker> Deserialize<'de> for Id<M> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct UuidVisitor<M> {
            phantom: PhantomData<M>,
        }

        impl<'de, M: Marker> serde::de::Visitor<'de> for UuidVisitor<M> {
            type Value = Id<M>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a UUID string or bytes")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                let uuid = Uuid::parse_str(value).map_err(E::custom)?;
                Ok(Id {
                    inner: uuid,
                    phantom: PhantomData,
                })
            }

            fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
                let uuid = Uuid::from_slice(value).map_err(E::custom)?;
                Ok(Id {
                    inner: uuid,
                    phantom: PhantomData,
                })
            }

            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                let uuid = Uuid::parse_str(&value).map_err(E::custom)?;
                Ok(Id {
                    inner: uuid,
                    phantom: PhantomData,
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                // Fallback for MessagePack interpreting bytes as sequence
                let mut bytes = Vec::new();
                while let Some(b) = seq.next_element::<u8>()? {
                    bytes.push(b);
                }
                let uuid = Uuid::from_slice(&bytes).map_err(serde::de::Error::custom)?;
                Ok(Id {
                    inner: uuid,
                    phantom: PhantomData,
                })
            }
        }

        // Use deserialize_any to seamlessly adapt to both JSON strings and
        // older msgpack bytes clients might still be sending.
        deserializer.deserialize_any(UuidVisitor {
            phantom: PhantomData,
        })
    }
}

impl<M: Marker> fmt::Debug for Id<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}Id({})", M::name(), self.inner)
    }
}

#[cfg(feature = "utoipa")]
mod schema {
    use utoipa::{
        openapi::{schema::Schema, RefOr},
        schema, PartialSchema, ToSchema,
    };

    use super::{Id, Marker};

    impl<M: Marker> PartialSchema for Id<M> {
        fn schema() -> utoipa::openapi::RefOr<Schema> {
            RefOr::T(Schema::Object(
                schema!(Uuid)
                    .title(Some("Uuid"))
                    .description(Some("A universally unique identifier."))
                    .build(),
            ))
        }
    }

    impl<M: Marker> ToSchema for Id<M> {}
}

impl<M: Marker> Clone for Id<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Marker> Copy for Id<M> {}

impl<M: Marker> Display for Id<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<M: Marker> From<Uuid> for Id<M> {
    fn from(inner: Uuid) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<M: Marker> From<Id<M>> for Uuid {
    fn from(val: Id<M>) -> Self {
        val.inner
    }
}

impl<M: Marker> Id<M> {
    pub fn new() -> Self {
        Uuid::now_v7().into()
    }

    pub fn into_inner(self) -> Uuid {
        self.into()
    }
}

impl<M: Marker> Default for Id<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Marker> FromStr for Id<M> {
    type Err = <Uuid as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<Uuid>()?.into())
    }
}

impl<M: Marker> TryInto<Time> for Id<M> {
    type Error = ();

    fn try_into(self) -> Result<Time, Self::Error> {
        let uuid: Uuid = self.into();
        uuid.get_timestamp().ok_or(())?.try_into().map_err(|_| ())
    }
}

impl<M: Marker> Deref for Id<M> {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<M: Marker + Ord + Eq> Identifier for Id<M> {}

macro_rules! genid {
    ($name:ident, $example:expr) => {
        pastey::paste! {
            #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
            #[non_exhaustive]
            pub enum [<Marker $name>] {}

            impl Marker for [<Marker $name>] {
                fn name() -> &'static str {
                    stringify!($name)
                }
            }

            impl private::Sealed for [<Marker $name>] {}

            pub type [<$name Id>] = Id<[<Marker $name>]>;
        }
    };
    ($name:ident) => {
        pastey::paste! {
            #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
            #[non_exhaustive]
            pub enum [<Marker $name>] {}

            impl Marker for [<Marker $name>] {
                fn name() -> &'static str {
                    stringify!($name)
                }
            }

            impl private::Sealed for [<Marker $name>] {}

            pub type [<$name Id>] = Id<[<Marker $name>]>;
        }
    };
}

// i might not need version ids for everything

genid!(Room, "00000000-0000-0000-0000-00000000room");
genid!(RoomVer, "00000000-0000-0000-0ver-00000000room");
genid!(Channel, "00000000-0000-0000-0000-00000channel");
genid!(ChannelVer, "00000000-0000-0000-0ver-00000channelver");
genid!(Message, "00000000-0000-0000-0000-00000message");
genid!(MessageVer, "00000000-0000-0000-0ver-00000message");
genid!(User, "00000000-0000-0000-0000-00000000user");
genid!(UserVer, "00000000-0000-0000-0ver-00000000user");
genid!(Role, "00000000-0000-0000-0000-00000000role");
genid!(RoleVer, "00000000-0000-0000-0ver-00000000role");
genid!(Media, "00000000-0000-0000-0000-0000000media");
genid!(Session, "00000000-0000-0000-0000-00000session");
// genid!(SessionVer, "00000000-0000-0000-0ver-00000session");
genid!(AuditLogEntry, "00000000-0000-0000-0000-0auditlogent");
genid!(Embed, "00000000-0000-0000-0new-0000000embed");
genid!(Tag, "00000000-0000-0000-0000-000000000tag");
genid!(Report, "00000000-0000-0000-0000-000modreport");
genid!(Redex, "00000000-0000-0000-0000-0000000redex");
genid!(Call, "00000000-0000-0000-0000-00000000call");
genid!(Emoji, "00000000-0000-0000-0000-0000000emoji");
genid!(Application, "00000000-0000-0000-0000-0application");
genid!(Notification, "00000000-0000-0000-0000-notification");
genid!(Sfu, "00000000-0000-0000-0000-000000000sfu");
genid!(AutomodRule, "00000000-0000-0000-0000-0automodrule");
genid!(Webhook, "00000000-0000-0000-0000-00000webhook");
genid!(CalendarEvent, "00000000-0000-0000-0000-calendarevent");
genid!(Harvest);
genid!(DocumentBranch);
genid!(DocumentTag);
genid!(Connection);

#[cfg(feature = "feat_interaction")]
genid!(Interaction, "00000000-0000-0000-0000-00interaction");

/// the user id of the server system user (aka root)
// hex translates to "root"
pub const SERVER_USER_ID: UserId = user_id!("00000000-0000-7000-0000-0000726f6f74");

/// the room id of the server system room
// hex translates to "server"
pub const SERVER_ROOM_ID: RoomId = room_id!("00000000-0000-7000-0000-736572766572");

/// the user id of the automod system user
// hex translates to "automod"
pub const AUTOMOD_USER_ID: UserId = user_id!("00000000-0000-7000-0061-75746f6d6f64");

/// the session id used for the admin token
// hex translates to "skeletonkey"
pub const SERVER_TOKEN_SESSION_ID: SessionId = session_id!("00000000-0073-6b65-6c65-746f6e6b6579");

/// server room role id for server admins
// hex translates to "admin"
pub const SERVER_ADMIN_ROLE_ID: RoleId = role_id!("00000000-0000-0000-0000-0061646d696e");

/// server room role id for registered users
// hex translates to "registered"
pub const SERVER_REGISTERED_ROLE_ID: RoleId = role_id!("00000000-0000-7265-6769-737465726564");
