use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Permission;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// TODO: derive macro
pub trait Diff<T> {
    fn changes(&self, other: &T) -> bool;
}

impl<T: PartialEq> Diff<T> for T {
    fn changes(&self, other: &T) -> bool {
        self != other
    }
}

impl<T: PartialEq> Diff<T> for Option<T> {
    fn changes(&self, other: &T) -> bool {
        self.as_ref().is_some_and(|s| s.changes(other))
    }
}

pub fn deserialize_default_true<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<bool>::deserialize(deserializer).map(|b| b.unwrap_or(true))
}

pub fn deserialize_sorted_permissions<'de, D>(deserializer: D) -> Result<Vec<Permission>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Vec::<Permission>::deserialize(deserializer).map(|mut v| {
        v.sort();
        v
    })
}

pub fn deserialize_sorted_permissions_option<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<Permission>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<Vec<Permission>>::deserialize(deserializer).map(|opt| {
        opt.map(|mut vec| {
            vec.sort();
            vec
        })
    })
}

// https://github.com/serde-rs/serde/issues/904
pub fn some_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

// TODO: swap all date/time types to this
/// A date, time, and timezone. Serialized to rfc3339.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Time(
    #[serde(
        serialize_with = "time::serde::rfc3339::serialize",
        deserialize_with = "time::serde::rfc3339::deserialize"
    )]
    time::OffsetDateTime,
);

impl Time {
    pub fn now_utc() -> Self {
        Self(time::OffsetDateTime::now_utc())
    }
}

pub fn time_rfc3339_option_serialize<S>(
    opt: &Option<time::OffsetDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    #[derive(Serialize)]
    struct Wrap(#[serde(serialize_with = "time::serde::rfc3339::serialize")] time::OffsetDateTime);

    match opt {
        Some(dt) => serializer.serialize_some(&Wrap(*dt)),
        None => serializer.serialize_none(),
    }
}

pub fn time_rfc3339_option_deserialize<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<time::OffsetDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrap(
        #[serde(deserialize_with = "time::serde::rfc3339::deserialize")] time::OffsetDateTime,
    );

    Option::<Wrap>::deserialize(deserializer).map(|o| o.map(|w| w.0))
}
