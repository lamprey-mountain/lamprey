use std::{
    ops::{Add, Deref, Sub},
    time::Duration,
};

use crate::v1::types::misc::duration::Duration as CommonDuration;

use time::{OffsetDateTime, PrimitiveDateTime};

/// A date, time, and timezone. Serialized to rfc3339.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time(OffsetDateTime);

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

impl Add<CommonDuration> for Time {
    type Output = Time;

    fn add(self, rhs: CommonDuration) -> Self::Output {
        self + std::time::Duration::from(rhs)
    }
}

impl Sub<Duration> for Time {
    type Output = Time;

    fn sub(self, rhs: Duration) -> Self::Output {
        Time::from(self.0 - rhs)
    }
}

impl Sub<CommonDuration> for Time {
    type Output = Time;

    fn sub(self, rhs: CommonDuration) -> Self::Output {
        self - std::time::Duration::from(rhs)
    }
}

impl Sub<Time> for Time {
    type Output = time::Duration;

    fn sub(self, rhs: Time) -> Self::Output {
        self.0 - rhs.0
    }
}

#[cfg(feature = "serde")]
mod _s {
    use super::*;
    use serde::{Deserialize, Serialize};

    impl Serialize for Time {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            if serializer.is_human_readable() {
                time::serde::rfc3339::serialize(&self.0, serializer)
            } else {
                serializer.serialize_i128(self.0.unix_timestamp_nanos())
            }
        }
    }

    impl<'de> Deserialize<'de> for Time {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct TimeVisitor;

            impl<'de> serde::de::Visitor<'de> for TimeVisitor {
                type Value = Time;

                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("an RFC3339 string or a unix timestamp in nanoseconds")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    OffsetDateTime::parse(v, &time::format_description::well_known::Rfc3339)
                        .map(Time)
                        .map_err(E::custom)
                }

                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_i128(v as i128)
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_i128(v as i128)
                }

                fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    OffsetDateTime::from_unix_timestamp_nanos(v)
                        .map(Time)
                        .map_err(E::custom)
                }

                fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    self.visit_i128(v as i128)
                }
            }

            deserializer.deserialize_any(TimeVisitor)
        }
    }
}

#[cfg(feature = "utoipa")]
mod _u {
    use super::*;

    use utoipa::{
        PartialSchema, ToSchema,
        openapi::{
            ObjectBuilder, RefOr,
            schema::{Schema, Type},
        },
    };

    impl PartialSchema for Time {
        fn schema() -> RefOr<Schema> {
            RefOr::T(
            ObjectBuilder::new()
                .schema_type(Type::String)
                .description(Some(
                    "A datetime in rfc3339 format for json.\n\nIs a i128 unix timestamp in nanoseconds for binary formats.",
                ))
                .examples([
                    serde_json::json!("2025-07-20T18:04:54.309625Z"),
                ])
                .build()
                .into()
            )
        }
    }

    impl ToSchema for Time {}
}
