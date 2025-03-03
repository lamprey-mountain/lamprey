use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize};

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

impl Deref for Time {
    type Target = time::OffsetDateTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Time {
    pub fn into_inner(self) -> time::OffsetDateTime {
        self.0
    }
}
