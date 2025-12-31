#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    pagination::PaginationDirection, util::some_option, CalendarEventId, ChannelId, UserId,
};

use super::util::Time;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarEvent {
    pub id: CalendarEventId,
    pub channel_id: ChannelId,
    pub creator_id: Option<UserId>,
    #[cfg_attr(feature = "utoipa", schema(max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(max = 64)))]
    pub title: String,
    #[cfg_attr(feature = "utoipa", schema(max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    pub description: Option<String>,
    #[cfg_attr(feature = "utoipa", schema(max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(max = 512)))]
    pub location: Option<String>,
    pub url: Option<Url>,

    /// the timezone that this event should be calculated in
    pub timezone: Option<Timezone>,

    pub recurrence: Option<Recurrence>,
    pub starts_at: Time,
    pub ends_at: Option<Time>,
}

/// a timezone
// TODO: validate? maybe allow only specific timezones?
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Timezone(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarEventCreate {
    #[cfg_attr(feature = "utoipa", schema(max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(max = 64)))]
    pub title: String,
    #[cfg_attr(feature = "utoipa", schema(max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    pub description: Option<String>,
    #[cfg_attr(feature = "utoipa", schema(max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(max = 512)))]
    pub location: Option<String>,
    pub url: Option<Url>,
    pub timezone: Option<Timezone>,
    pub recurrence: Option<Recurrence>,
    pub starts_at: Time,
    pub ends_at: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarEventPatch {
    #[cfg_attr(feature = "utoipa", schema(max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(max = 64)))]
    pub title: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,

    #[cfg_attr(feature = "utoipa", schema(max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(max = 512)))]
    #[serde(default, deserialize_with = "some_option")]
    pub location: Option<Option<String>>,

    #[serde(default, deserialize_with = "some_option")]
    pub url: Option<Option<Url>>,

    pub starts_at: Option<Time>,

    #[serde(default, deserialize_with = "some_option")]
    pub ends_at: Option<Option<Time>>,
    // NOTE: undecided features
    // how will moving events between channels work? what happens to rsvps for users who can no longer see an event?
    // pub channel_id: Option<ChannelId>,
    //
    // how will ceruccence work with event overwrites?
    // pub recurrence: Option<Option<Recurrence>>,
}

/// an overwrite to a calendar event instance
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarOverwrite {
    /// the sequence number of this instance
    pub seq: u64,
    pub event_id: CalendarEventId,

    #[cfg_attr(feature = "utoipa", schema(max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(max = 64)))]
    pub title: Option<String>,

    /// shown before the description
    #[cfg_attr(feature = "utoipa", schema(max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    pub extra_description: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(max = 512)))]
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarOverwritePut {
    #[cfg_attr(feature = "utoipa", schema(max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(max = 64)))]
    pub title: Option<String>,

    /// shown before the description
    #[cfg_attr(feature = "utoipa", schema(max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    pub extra_description: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(max = 512)))]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarEventListQuery {
    #[cfg_attr(feature = "validator", validate(range(max = 1024)))]
    pub limit: Option<u16>,
    pub from: Option<CalendarEventId>,
    pub to: Option<CalendarEventId>,
    pub dir: Option<PaginationDirection>,
    pub from_time: Option<Time>,
    pub to_time: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Recurrence {
    /// how often to recur
    pub frequency: RecurrenceFrequency,

    /// only repeat on these days of the week
    #[serde(default)]
    pub by_weekday: Vec<DayOfWeek>,

    /// only repeat on these days of the month
    #[serde(default)]
    pub by_month_day: Vec<u8>,

    /// when to end
    pub range: RecurrenceRange,

    /// repeat every n (days/weeks/months/years)
    pub interval: u32,

    #[serde(default)]
    pub except_date: Vec<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum RecurrenceRange {
    /// repeat this event forever
    Infinite,

    /// repeat this event n times
    Count { count: u32 },

    /// repeat this event until this time
    Until { time: Time },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// a day of the week
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Recurrence {
    /// validate this rule (eg. if the constraints are valid)
    ///
    /// on error, returns a list of error messages
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = vec![];
        if self.interval == 0 {
            errors.push("Interval must be at least 1".to_string());
        }

        // by_weekday only valid for Weekly and Monthly
        if !self.by_weekday.is_empty() {
            if !matches!(
                self.frequency,
                RecurrenceFrequency::Weekly | RecurrenceFrequency::Monthly
            ) {
                errors.push(
                    "by_weekday is only valid for Weekly and Monthly frequency".to_string(),
                );
            }
        }

        // by_month_day only valid for Monthly, Yearly
        if !self.by_month_day.is_empty() {
            if !matches!(
                self.frequency,
                RecurrenceFrequency::Monthly | RecurrenceFrequency::Yearly
            ) {
                errors.push(
                    "by_month_day is only valid for Monthly and Yearly frequency".to_string(),
                );
            }

            // range 1..=31
            for day in &self.by_month_day {
                if *day < 1 || *day > 31 {
                    errors.push(format!(
                        "by_month_day values must be between 1 and 31, found {}",
                        day
                    ));
                }
            }
        }

        // by_weekday no duplicates
        let unique_weekdays: HashSet<_> = self.by_weekday.iter().collect();
        if unique_weekdays.len() != self.by_weekday.len() {
            errors.push("by_weekday must not contain duplicates".to_string());
        }

        // Count >= 1
        if let RecurrenceRange::Count { count } = self.range {
            if count < 1 {
                errors.push("Recurrence count must be at least 1".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// convert to a rfc rrule string
    pub fn to_rrule(&self) -> String {
        todo!()
    }

    /// if this event ends, gets the last day this series ends on
    pub fn series_ends_at(&self) -> Option<Time> {
        todo!()
    }

    /// if this event ends, gets the number of events in this series
    pub fn series_count(&self) -> Option<u64> {
        todo!()
    }

    /// calculate the default start date/time of the nth event
    pub fn nth_event_starts_at(&self, seq: u64) -> Option<Time> {
        todo!()
    }

    /// calculate the default end date/time of the nth event
    pub fn nth_event_ends_at(&self, seq: u64) -> Option<Time> {
        todo!()
    }
}

impl CalendarEvent {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = vec![];

        if let Some(ends_at) = self.ends_at {
            if ends_at <= self.starts_at {
                errors.push("ends_at must be after starts_at".to_string());
            }
        }

        if let Some(recurrence) = &self.recurrence {
            if let Err(rec_errors) = recurrence.validate() {
                errors.extend(rec_errors);
            }

            if let RecurrenceRange::Until { time } = recurrence.range {
                let end_time = self.ends_at.unwrap_or(self.starts_at);
                if time <= end_time {
                    errors.push(
                        "Recurrence until time must be after the event end time".to_string(),
                    );
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// calculate the duration of this event in milliseconds
    pub fn duration(&self) -> Option<u64> {
        todo!()
    }
}
