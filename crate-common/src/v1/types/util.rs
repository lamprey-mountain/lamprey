use serde::{Deserialize, Deserializer};

pub mod truncate;

// TEMP: pub use here for compatibility
pub use super::misc::Time;

// TODO: derive macro
// NOTE: maybe it should be the other way around?
// NOTE: maybe i should use associated types instead of generics
pub trait Diff<T> {
    fn changes(&self, other: &T) -> bool;
    // fn apply(self, other: T) -> T;
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

pub fn deserialize_sorted<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Ord,
{
    Vec::<T>::deserialize(deserializer).map(|mut v| {
        v.sort();
        v
    })
}

pub fn deserialize_sorted_option<'de, D, T>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Ord,
{
    Option::<Vec<T>>::deserialize(deserializer).map(|opt| {
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
