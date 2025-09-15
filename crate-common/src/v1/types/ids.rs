use core::fmt;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::str::FromStr;
use std::{fmt::Display, marker::PhantomData};
use uuid::{uuid, Uuid};

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

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<M: Marker> {
    inner: Uuid,

    #[serde(skip)]
    phantom: PhantomData<M>,
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
        self.inner.into()
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
genid!(Thread, "00000000-0000-0000-0000-000000thread");
genid!(ThreadVer, "00000000-0000-0000-0ver-000000thread");
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
genid!(TagVer, "00000000-0000-0000-0ver-000000000tag");
genid!(Report, "00000000-0000-0000-0000-000modreport");
genid!(Redex, "00000000-0000-0000-0000-0000000redex");
genid!(Call, "00000000-0000-0000-0000-00000000call");
genid!(Emoji, "00000000-0000-0000-0000-0000000emoji");
genid!(Application);

// genid!(Region); // not a uuid?
genid!(Server); // rename? Worker, Host
genid!(Livestream);
genid!(RtcPeer);

genid!(Notification);

pub const SERVER_USER_ID: UserId = Id {
    inner: uuid!("00000000-0000-7000-0000-0000726f6f74"),
    phantom: std::marker::PhantomData,
};

pub const SERVER_ROOM_ID: RoomId = Id {
    inner: uuid!("00000000-0000-7000-0000-736572766572"),
    phantom: std::marker::PhantomData,
};
