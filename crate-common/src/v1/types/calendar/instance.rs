use std::fmt;

use lamprey_macros::record;
use url::Url;

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use crate::v1::{
    routes::{PathParam, PathParamError},
    types::{CalendarEventId, misc::Time},
};

/// the sequence number of a calendar event instance
#[record]
#[derive(Copy, PartialEq, Eq)]
pub struct CalendarInstanceSeq(u64);

/// the status of a calendar event instance
#[record]
#[derive(Copy, PartialEq, Eq)]
pub enum CalendarInstanceStatus {
    /// event has not started yet
    Scheduled,

    /// event is active
    Active,

    /// event has ended
    Completed,

    /// event is canceled
    Canceled,
}

/// an instance of a calendar event
///
/// used for recurring events
#[record]
pub struct CalendarInstance {
    /// the id of the calendar event this is for
    pub event_id: CalendarEventId,

    /// the sequence number of this instance
    pub seq: CalendarInstanceSeq,

    /// the status of this instance
    pub status: CalendarInstanceStatus,

    #[serde(flatten)]
    pub overwrite: Option<CalendarOverwrite>,
}

/// an overwrite to a calendar event instance
// Option<Option<T>>: no overwrite, set to none, set to value
#[record]
pub struct CalendarOverwrite {
    /// set the title for this event instance
    #[schema(max_length = 64)]
    #[validate(length(max = 64))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// extra description to show for this event instance
    #[schema(max_length = 4096)]
    #[validate(length(max = 4096))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_description: Option<String>,

    /// overwrite the location for this event instance
    #[schema(max_length = 512)]
    #[validate(length(max = 512))]
    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub location: Option<Option<String>>,

    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub url: Option<Option<Url>>,

    /// Overwrite the start time for this event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<Time>,

    /// Overwrite the end time for this event
    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub ends_at: Option<Option<Time>>,

    /// whether this event is cancelled
    #[serde(default, skip_serializing_if = "is_false")]
    pub cancelled: bool,
}

impl CalendarInstanceSeq {
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn zero() -> Self {
        Self(0)
    }
}

impl From<u64> for CalendarInstanceSeq {
    fn from(seq: u64) -> Self {
        Self(seq)
    }
}

impl From<CalendarInstanceSeq> for u64 {
    fn from(seq: CalendarInstanceSeq) -> Self {
        seq.0
    }
}

impl PathParam for CalendarInstanceSeq {
    fn from_path_param(s: &str) -> Result<Self, PathParamError> {
        let seq = s
            .parse()
            .map_err(|_| PathParamError(format!("invalid CalendarOverwriteSeq: {}", s)))?;
        Ok(Self(seq))
    }
}

impl fmt::Display for CalendarInstanceSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn is_false(b: &bool) -> bool {
    !b
}
