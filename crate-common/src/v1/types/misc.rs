use std::ops::Deref;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// maybe i need a version that explicitly drops some brightness and saturation (keeping hue)
/// a color
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(untagged)]
pub enum Color {
    // Named(ColorNamed),
    /// sRGB (not linear) compatible with css
    // FIXME: this should be a hex code, not arbitrary string
    Srgb(String),
    // with alpha as separate ver? Srgba(String),
}

/// a color that changes depending on theme
/// color names currently unstable and may change
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ColorThemed {
    /// default text color
    FgMain,

    /// low priority text color
    FgDim,

    /// default background color (for this item)
    BgMain,

    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Orange,
    Teal,
}

/// color with semantic meaning
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ColorSemantic {
    /// something worth pointing out
    Note,

    /// something with useful information
    Info,

    /// instructions or tips
    Help,

    /// very important to read, generic
    Important,

    /// very important to read, bad things happen if you don't
    Warning,

    /// very important to read, dangerous things happen if you don't
    Danger,

    /// something went wrong
    Error,

    /// something went right
    Success,
}

impl Color {
    pub fn from_hex_string(s: String) -> Color {
        // TODO: ensure hex
        Color::Srgb(s)
    }
}

impl AsRef<str> for Color {
    fn as_ref(&self) -> &str {
        match self {
            Color::Srgb(c) => c.as_str(),
        }
    }
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
