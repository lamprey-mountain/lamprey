use lamprey_macros::record;
use url::Url;

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use crate::v1::types::{
    CalendarEventId, ChannelId, UserId,
    calendar::{Recurrence, RecurrenceLimit, Timezone},
    error::{ApiError, ErrorCode, ErrorField, ErrorFieldType},
    misc::Time,
};

#[record]
pub struct CalendarEvent {
    pub id: CalendarEventId,
    pub channel_id: ChannelId,
    pub creator_id: Option<UserId>,
    #[schema(max_length = 64)]
    #[validate(length(max = 64))]
    pub title: String,
    #[schema(max_length = 4096)]
    #[validate(length(max = 4096))]
    pub description: Option<String>,
    #[schema(max_length = 512)]
    #[validate(length(max = 512))]
    pub location: Option<String>,
    pub url: Option<Url>,

    /// the timezone that this event should be calculated in
    pub timezone: Option<Timezone>,

    pub recurrence: Option<Recurrence>,
    pub starts_at: Time,
    pub ends_at: Option<Time>,
}

#[record]
pub struct CalendarEventCreate {
    #[schema(max_length = 64)]
    #[validate(length(max = 64))]
    pub title: String,
    #[schema(max_length = 4096)]
    #[validate(length(max = 4096))]
    pub description: Option<String>,
    #[schema(max_length = 512)]
    #[validate(length(max = 512))]
    pub location: Option<String>,
    pub url: Option<Url>,
    pub timezone: Option<Timezone>,
    pub recurrence: Option<Recurrence>,
    pub starts_at: Time,
    pub ends_at: Option<Time>,
}

// TODO: rename to CalendarEventUpdate
#[record]
pub struct CalendarEventPatch {
    #[schema(max_length = 64)]
    #[validate(length(max = 64))]
    pub title: Option<String>,

    #[schema(max_length = 4096)]
    #[validate(length(max = 4096))]
    #[serde(default, deserialize_with = "some_option")]
    pub description: Option<Option<String>>,

    #[schema(max_length = 512)]
    #[validate(length(max = 512))]
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
