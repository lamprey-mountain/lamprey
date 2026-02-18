#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    error::{ApiError, ErrorCode, ErrorField, ErrorFieldType},
    misc::Color,
    pagination::PaginationDirection,
    CalendarEventId, ChannelId, RoomMember, User, UserId,
};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use super::util::{Diff, Time};

/// channel metadata for a calendar
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Calendar {
    /// the color of this calendar
    pub color: Option<Color>,

    /// the default timezone events in this calendar should be created in
    pub default_timezone: Timezone,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

impl CalendarEventCreate {
    pub fn validate(&self) -> Result<(), ApiError> {
        let mut fields = vec![];

        if let Some(ends_at) = self.ends_at {
            if ends_at <= self.starts_at {
                fields.push(ErrorField {
                    key: vec!["ends_at".to_owned()],
                    message: "ends_at must be after starts_at".to_owned(),
                    ty: ErrorFieldType::Other,
                });
            }
        }

        if let Some(recurrence) = &self.recurrence {
            if let Err(rec_errors) = recurrence.validate() {
                fields.extend(rec_errors);
            }

            if let RecurrenceRange::Until { time } = recurrence.range {
                let end_time = self.ends_at.unwrap_or(self.starts_at);
                if time <= end_time {
                    fields.push(ErrorField {
                        key: vec![
                            "recurrence".to_owned(),
                            "range".to_owned(),
                            "until".to_owned(),
                        ],
                        message: "Recurrence until time must be after the event end time"
                            .to_owned(),
                        ty: ErrorFieldType::Other,
                    });
                }
            }
        }

        if fields.is_empty() {
            Ok(())
        } else {
            let mut err = ApiError::from_code(ErrorCode::InvalidData);
            err.fields = fields;
            Err(err)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarEventPatch {
    #[cfg_attr(feature = "utoipa", schema(max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(max = 64)))]
    pub title: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub description: Option<Option<String>>,

    #[cfg_attr(feature = "utoipa", schema(max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(max = 512)))]
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub location: Option<Option<String>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub url: Option<Option<Url>>,

    pub starts_at: Option<Time>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub ends_at: Option<Option<Time>>,
    // NOTE: undecided features
    // how will moving events between channels work? what happens to rsvps for users who can no longer see an event?
    // pub channel_id: Option<ChannelId>,
    //
    // how will ceruccence work with event overwrites?
    // pub recurrence: Option<Option<Recurrence>>,
}

impl CalendarEventPatch {
    pub fn validate(&self) -> Result<(), ApiError> {
        let mut fields = vec![];

        if let Some(starts_at) = self.starts_at {
            if let Some(ends_at) = self.ends_at.flatten() {
                if ends_at <= starts_at {
                    fields.push(ErrorField {
                        key: vec!["ends_at".to_owned()],
                        message: "ends_at must be after starts_at".to_string(),
                        ty: ErrorFieldType::Other,
                    });
                }
            }
        }

        if fields.is_empty() {
            Ok(())
        } else {
            let mut err = ApiError::from_code(ErrorCode::InvalidData);
            err.fields = fields;
            Err(err)
        }
    }
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
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub location: Option<Option<String>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub url: Option<Option<Url>>,

    /// Overwrite the start time for this event
    pub starts_at: Option<Time>,

    /// Overwrite the end time for this event
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
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
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub location: Option<Option<String>>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub url: Option<Option<Url>>,

    /// Overwrite the start time for this event
    pub starts_at: Option<Time>,

    /// Overwrite the end time for this event
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub ends_at: Option<Option<Time>>,

    /// if this event is cancelled
    pub cancelled: Option<bool>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Recurrence {
    /// how often to recur
    pub frequency: RecurrenceFrequency,

    /// only repeat on these days of the week
    #[cfg_attr(feature = "serde", serde(default))]
    pub by_weekday: Vec<DayOfWeek>,

    /// only repeat on these days of the month
    #[cfg_attr(feature = "serde", serde(default))]
    pub by_month_day: Vec<u8>,

    /// when to end
    pub range: RecurrenceRange,

    /// repeat every n (days/weeks/months/years)
    pub interval: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum RecurrenceRange {
    /// repeat this event forever
    Infinite,

    /// repeat this event n times
    Count { count: u32 },

    /// repeat this event until this time
    Until { time: Time },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// a day of the week
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    pub fn validate(&self) -> Result<(), Vec<ErrorField>> {
        let mut errors = vec![];
        if self.interval == 0 {
            errors.push(ErrorField {
                key: vec!["interval".to_owned()],
                message: "Interval must be at least 1".to_owned(),
                ty: ErrorFieldType::Range {
                    min: Some(1),
                    max: None,
                },
            });
        }

        // by_weekday only valid for Weekly and Monthly
        if !self.by_weekday.is_empty() {
            if !matches!(
                self.frequency,
                RecurrenceFrequency::Weekly | RecurrenceFrequency::Monthly
            ) {
                errors.push(ErrorField {
                    key: vec!["by_weekday".to_owned()],
                    message: "by_weekday is only valid for Weekly and Monthly frequency".to_owned(),
                    ty: ErrorFieldType::Other,
                });
            }
        }

        // by_month_day only valid for Monthly, Yearly
        if !self.by_month_day.is_empty() {
            if !matches!(
                self.frequency,
                RecurrenceFrequency::Monthly | RecurrenceFrequency::Yearly
            ) {
                errors.push(ErrorField {
                    key: vec!["by_month_day".to_owned()],
                    message: "by_month_day is only valid for Monthly and Yearly frequency"
                        .to_owned(),
                    ty: ErrorFieldType::Other,
                });
            }

            // range 1..=31
            for day in &self.by_month_day {
                if *day < 1 || *day > 31 {
                    errors.push(ErrorField {
                        key: vec!["by_month_day".to_owned()],
                        message: format!(
                            "by_month_day values must be between 1 and 31, found {}",
                            day
                        ),
                        ty: ErrorFieldType::Range {
                            min: Some(1),
                            max: Some(31),
                        },
                    });
                }
            }
        }

        // by_weekday no duplicates
        let unique_weekdays: HashSet<_> = self.by_weekday.iter().collect();
        if unique_weekdays.len() != self.by_weekday.len() {
            errors.push(ErrorField {
                key: vec!["by_weekday".to_owned()],
                message: "by_weekday must not contain duplicates".to_owned(),
                ty: ErrorFieldType::Other,
            });
        }

        // Count >= 1
        if let RecurrenceRange::Count { count } = self.range {
            if count < 1 {
                errors.push(ErrorField {
                    key: vec!["range".to_owned(), "count".to_owned()],
                    message: "Recurrence count must be at least 1".to_owned(),
                    ty: ErrorFieldType::Range {
                        min: Some(1),
                        max: None,
                    },
                });
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
        let mut rrule = vec![];

        let freq = match self.frequency {
            RecurrenceFrequency::Daily => "DAILY",
            RecurrenceFrequency::Weekly => "WEEKLY",
            RecurrenceFrequency::Monthly => "MONTHLY",
            RecurrenceFrequency::Yearly => "YEARLY",
        };
        rrule.push(format!("FREQ={}", freq));

        rrule.push(format!("INTERVAL={}", self.interval));

        if !self.by_weekday.is_empty() {
            let days: Vec<&str> = self
                .by_weekday
                .iter()
                .map(|d| match d {
                    DayOfWeek::Monday => "MO",
                    DayOfWeek::Tuesday => "TU",
                    DayOfWeek::Wednesday => "WE",
                    DayOfWeek::Thursday => "TH",
                    DayOfWeek::Friday => "FR",
                    DayOfWeek::Saturday => "SA",
                    DayOfWeek::Sunday => "SU",
                })
                .collect();
            rrule.push(format!("BYDAY={}", days.join(",")));
        }

        if !self.by_month_day.is_empty() {
            let days: Vec<String> = self.by_month_day.iter().map(|d| d.to_string()).collect();
            rrule.push(format!("BYMONTHDAY={}", days.join(",")));
        }

        match &self.range {
            RecurrenceRange::Count { count } => {
                rrule.push(format!("COUNT={}", count));
            }
            RecurrenceRange::Until { time } => {
                let dt = time.to_offset(time::UtcOffset::UTC);
                let fmt =
                    time::format_description::parse("[year][month][day]T[hour][minute][second]Z")
                        .unwrap();
                rrule.push(format!("UNTIL={}", dt.format(&fmt).unwrap()));
            }
            RecurrenceRange::Infinite => {}
        }

        rrule.join(";")
    }
}

impl CalendarEvent {
    pub fn validate(&self) -> Result<(), Vec<ErrorField>> {
        let mut errors = vec![];

        if let Some(ends_at) = self.ends_at {
            if ends_at <= self.starts_at {
                errors.push(ErrorField {
                    key: vec!["ends_at".to_owned()],
                    message: "ends_at must be after starts_at".to_owned(),
                    ty: ErrorFieldType::Other,
                });
            }
        }

        if let Some(recurrence) = &self.recurrence {
            if let Err(rec_errors) = recurrence.validate() {
                errors.extend(rec_errors);
            }

            if let RecurrenceRange::Until { time } = recurrence.range {
                let end_time = self.ends_at.unwrap_or(self.starts_at);
                if time <= end_time {
                    errors.push(ErrorField {
                        key: vec![
                            "recurrence".to_owned(),
                            "range".to_owned(),
                            "until".to_owned(),
                        ],
                        message: "Recurrence until time must be after the event end time"
                            .to_owned(),
                        ty: ErrorFieldType::Other,
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema, IntoParams))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarEventParticipantQuery {
    /// whether to include user and member
    #[cfg_attr(feature = "serde", serde(default))]
    pub include_member: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CalendarEventParticipant {
    pub user_id: UserId,
    pub status: CalendarRsvpStatus,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub user: Option<User>,

    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub member: Option<RoomMember>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CalendarRsvpStatus {
    Interested,
    Uninterested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarEventParticipantPut {
    pub status: CalendarRsvpStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarPatch {
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "some_option"))]
    pub color: Option<Option<Color>>,
    pub default_timezone: Option<Timezone>,
}

impl Diff<Calendar> for CalendarPatch {
    fn changes(&self, other: &Calendar) -> bool {
        self.default_timezone.changes(&other.default_timezone) || self.color.changes(&other.color)
    }
}
