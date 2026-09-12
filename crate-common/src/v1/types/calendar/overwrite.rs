use lamprey_macros::record;
use url::Url;

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use crate::v1::types::{CalendarEventId, misc::Time};

// TODO: impl and use
// pub struct CalendarOverwriteSeq(u64);

/// an overwrite to a calendar event instance
#[record]
pub struct CalendarOverwrite {
    /// the sequence number of this instance
    pub seq: u64,
    pub event_id: CalendarEventId,

    #[schema(max_length = 64)]
    #[validate(length(max = 64))]
    pub title: Option<String>,

    /// shown before the description
    #[schema(max_length = 4096)]
    #[validate(length(max = 4096))]
    pub extra_description: Option<String>,

    #[schema(max_length = 512)]
    #[validate(length(max = 512))]
    #[serde(default, deserialize_with = "some_option")]
    pub location: Option<Option<String>>,

    #[serde(default, deserialize_with = "some_option")]
    pub url: Option<Option<Url>>,

    /// Overwrite the start time for this event
    pub starts_at: Option<Time>,

    /// Overwrite the end time for this event
    #[serde(default, deserialize_with = "some_option")]
    pub ends_at: Option<Option<Time>>,

    /// if this event is cancelled
    pub cancelled: bool,
}

// TODO: rename to CalendarOverwriteCreate
#[record]
pub struct CalendarOverwritePut {
    #[schema(max_length = 64)]
    #[validate(length(max = 64))]
    pub title: Option<String>,

    /// shown before the description
    #[schema(max_length = 4096)]
    #[validate(length(max = 4096))]
    pub extra_description: Option<String>,

    #[schema(max_length = 512)]
    #[validate(length(max = 512))]
    #[serde(default, deserialize_with = "some_option")]
    pub location: Option<Option<String>>,

    #[serde(default, deserialize_with = "some_option")]
    pub url: Option<Option<Url>>,

    /// Overwrite the start time for this event
    pub starts_at: Option<Time>,

    /// Overwrite the end time for this event
    #[serde(default, deserialize_with = "some_option")]
    pub ends_at: Option<Option<Time>>,

    /// if this event is cancelled
    pub cancelled: Option<bool>,
}
