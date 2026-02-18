use std::{
    ops::{Add, Deref, Sub},
    time::Duration,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// TODO: swap all date/time types to this
/// A date, time, and timezone. Serialized to rfc3339.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Time(
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "time::serde::rfc3339::serialize",
            deserialize_with = "time::serde::rfc3339::deserialize"
        )
    )]
    OffsetDateTime,
);

impl Time {
    pub fn now_utc() -> Self {
        Self(OffsetDateTime::now_utc())
    }
}

impl Deref for Time {
    type Target = OffsetDateTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Time {
    pub fn into_inner(self) -> OffsetDateTime {
        self.0
    }
}

impl TryInto<Time> for uuid::Timestamp {
    type Error = time::error::ComponentRange;

    fn try_into(self) -> Result<Time, Self::Error> {
        let (secs, nanos) = self.to_unix();
        let ts = secs as i128 * 1000000000 + nanos as i128;
        Ok(Time(OffsetDateTime::from_unix_timestamp_nanos(ts)?))
    }
}

impl From<OffsetDateTime> for Time {
    fn from(value: OffsetDateTime) -> Self {
        Time(value)
    }
}

impl From<PrimitiveDateTime> for Time {
    fn from(value: PrimitiveDateTime) -> Self {
        value.assume_utc().into()
    }
}

impl From<Time> for PrimitiveDateTime {
    fn from(value: Time) -> Self {
        PrimitiveDateTime::new(value.0.date(), value.0.time())
    }
}

impl Add<Duration> for Time {
    type Output = Time;

    fn add(self, rhs: Duration) -> Self::Output {
        Time::from(self.0 + rhs)
    }
}

impl Sub<Duration> for Time {
    type Output = Time;

    fn sub(self, rhs: Duration) -> Self::Output {
        Time::from(self.0 - rhs)
    }
}
