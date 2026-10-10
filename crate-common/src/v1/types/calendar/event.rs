use lamprey_macros::{Diff, record};
use url::Url;

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use crate::v1::types::{
    CalendarEventId, ChannelId, MediaId, PaginationDirection, RoomId, UserId,
    calendar::{CalendarParticipantCounts, Recurrence, RecurrenceLimit, Timezone},
    error::{ApiError, ErrorCode, ErrorField, ErrorFieldType},
    misc::Time,
};

/// a calendar event
#[record]
pub struct CalendarEvent {
    /// unique identifier for this calendar event
    pub id: CalendarEventId,

    /// the id of the calendar channel that this event was created in
    pub channel_id: ChannelId,

    /// the id of the room this event is in
    // TODO: make this not an option
    pub room_id: Option<RoomId>,

    /// the id of the user who created this event
    // TODO: make this not an option
    pub creator_id: Option<UserId>,

    /// the cover image for this calendar event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_id: Option<MediaId>,

    /// name for this event
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub title: String,

    /// description for this event. supports markdown.
    #[schema(min_length = 1, max_length = 4096)]
    #[validate(length(min = 1, max = 4096))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// location where this event will take place
    #[schema(min_length = 1, max_length = 512)]
    #[validate(length(min = 1, max = 512))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// a url for this event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,

    /// the timezone that this event should be calculated in
    ///
    /// if unset, default to the channel's timezone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<Timezone>,

    /// how this event should recur
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence: Option<Recurrence>,

    /// when this event starts
    pub starts_at: Time,

    /// when this event ends
    ///
    /// if unset, this event describes a point in time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<Time>,

    /// whether this event lasts all day
    ///
    /// if true, use dates rather than datetimes for `starts_at`/`ends_at`.
    // TODO: skip serializing if is false
    pub all_day: bool,

    pub participant_counts: CalendarParticipantCounts,
}

#[record]
pub struct CalendarEventCreate {
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub title: String,
    #[schema(min_length = 1, max_length = 4096)]
    #[validate(length(min = 1, max = 4096))]
    pub description: Option<String>,
    #[schema(min_length = 1, max_length = 512)]
    #[validate(length(min = 1, max = 512))]
    pub location: Option<String>,
    pub url: Option<Url>,
    pub timezone: Option<Timezone>,
    pub recurrence: Option<Recurrence>,
    pub starts_at: Time,
    pub ends_at: Option<Time>,
    pub media_id: Option<MediaId>,
    pub all_day: bool,
}

#[record]
#[derive(Default, Diff)]
#[diff(target = "CalendarEvent")]
pub struct CalendarEventUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(min_length = 1, max_length = 64)]
    #[validate(length(min = 1, max = 64))]
    pub title: Option<String>,

    #[schema(min_length = 1, max_length = 4096)]
    #[validate(length(min = 1, max = 4096))]
    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<Option<String>>,

    #[schema(min_length = 1, max_length = 512)]
    #[validate(length(min = 1, max = 512))]
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<Time>,

    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub ends_at: Option<Option<Time>>,

    /// move this event to another channel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,

    /// change the recurrence rule for this calendar event
    ///
    /// recurrence has restrictions on whether it can be changed and will result in a 409 conflict if you aren't careful
    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub recurrence: Option<Option<Recurrence>>,

    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub timezone: Option<Option<Timezone>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_day: Option<bool>,

    #[serde(
        default,
        deserialize_with = "some_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub media_id: Option<Option<MediaId>>,
}

#[record(params)]
#[derive(Default)]
pub struct CalendarEventListQuery {
    /// return at most this many calendar events
    #[validate(range(max = 1024))]
    pub limit: Option<u16>,

    pub from: Option<CalendarEventId>,
    pub to: Option<CalendarEventId>,
    pub dir: Option<PaginationDirection>,

    /// only return calendar events that start after this time
    pub from_time: Option<Time>,

    /// only return calendar events that end before this time
    pub to_time: Option<Time>,
}

#[record(params)]
#[derive(Default)]
pub struct CalendarExportQuery {
    /// authentication token
    pub token: Option<String>,

    /// whether to download the calendar
    // TODO: set Content-Disposition: attachment; filename="calendar.ics"
    #[serde(default)]
    pub download: bool,
}

#[record]
pub struct CalendarEventList {
    pub events: Vec<CalendarEvent>,
    // pub cursor: Option<CalendarEventId>,
}

impl CalendarEventList {
    // TODO: start time, end time
}

// TODO: impl Validator instead of custom validate fns

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

            if let RecurrenceLimit::Until { time } = recurrence.limit {
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

impl CalendarEventUpdate {
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

            if let RecurrenceLimit::Until { time } = recurrence.limit {
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

impl CalendarEventCreate {
    /// create a new calendar event
    pub fn new(starts_at: Time, title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            location: None,
            url: None,
            timezone: None,
            recurrence: None,
            starts_at,
            ends_at: None,
            media_id: None,
            all_day: false,
        }
    }

    pub fn ends(mut self, ends_at: impl Into<Time>) -> Self {
        self.ends_at = Some(ends_at.into());
        self
    }

    pub fn description(mut self, description: impl Into<Option<String>>) -> Self {
        self.description = description.into();
        self
    }

    pub fn location(mut self, location: impl Into<Option<String>>) -> Self {
        self.location = location.into();
        self
    }

    pub fn url(mut self, url: impl Into<Option<Url>>) -> Self {
        self.url = url.into();
        self
    }

    pub fn media(mut self, media_id: impl Into<Option<MediaId>>) -> Self {
        self.media_id = media_id.into();
        self
    }

    pub fn all_day(mut self, all_day: bool) -> Self {
        self.all_day = all_day;
        self
    }

    pub fn timezone(mut self, timezone: Timezone) -> Self {
        self.timezone = Some(timezone);
        self
    }

    pub fn recurrence(mut self, recurrence: Recurrence) -> Self {
        self.recurrence = Some(recurrence);
        self
    }
}
