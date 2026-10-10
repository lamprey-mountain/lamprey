use std::collections::HashSet;

use lamprey_macros::record;
use time::format_description::BorrowedFormatItem;

#[cfg(feature = "serde")]
use crate::v1::types::util::deserialize_sorted;

use crate::v1::types::{
    error::{ErrorField, ErrorFieldType},
    misc::Time,
};

// TODO: add by_year_day
// TODO: add by_n_weekday
// TODO: add by_month
#[record]
#[derive(PartialEq, Eq)]
pub struct Recurrence {
    /// how often to recur
    pub frequency: RecurrenceFrequency,

    /// only repeat on these days of the week
    ///
    /// only usable with [`RecurrenceFrequency::Weekly`] or [`RecurrenceFrequency::Monthly`]
    #[serde(default, deserialize_with = "deserialize_sorted")]
    pub by_weekday: Vec<DayOfWeek>,

    /// only repeat on these days of the month
    ///
    /// only usable with [`RecurrenceFrequency::Monthly`] or [`RecurrenceFrequency::Yearly`]
    #[serde(default, deserialize_with = "deserialize_sorted")]
    pub by_month_day: Vec<u8>,

    /// when to end
    #[serde(default)]
    pub limit: RecurrenceLimit,

    /// repeat every n (days/weeks/months/years)
    #[serde(default = "const_one")]
    pub interval: u32,
}

#[record]
#[derive(Default, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum RecurrenceLimit {
    /// repeat this event forever
    #[default]
    Infinite,

    /// repeat this event n times
    Count { count: u32 },

    /// repeat this event until this time
    Until { time: Time },
}

#[record]
#[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// a day of the week
#[record]
#[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

fn const_one() -> u32 {
    1
}

impl RecurrenceFrequency {
    fn as_rrule(&self) -> &'static str {
        match self {
            Self::Daily => "DAILY",
            Self::Weekly => "WEEKLY",
            Self::Monthly => "MONTHLY",
            Self::Yearly => "YEARLY",
        }
    }
}

impl DayOfWeek {
    pub fn is_weekend(&self) -> bool {
        matches!(self, Self::Saturday | Self::Sunday)
    }

    pub fn is_weekday(&self) -> bool {
        !self.is_weekend()
    }

    fn as_rrule(&self) -> &'static str {
        match self {
            Self::Monday => "MO",
            Self::Tuesday => "TU",
            Self::Wednesday => "WE",
            Self::Thursday => "TH",
            Self::Friday => "FR",
            Self::Saturday => "SA",
            Self::Sunday => "SU",
        }
    }
}

// TODO: impl validator::Validate for this
// TODO: impl parsing from rrule
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
        if let RecurrenceLimit::Count { count } = self.limit {
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

    /// convert to a [rfc 5545](https://www.rfc-editor.org/info/rfc5545/) rrule string
    pub fn to_rrule(&self) -> Result<String, time::error::Format> {
        let mut parts = vec![
            format!("FREQ={}", self.frequency.as_rrule()),
            format!("INTERVAL={}", self.interval),
        ];

        if !self.by_weekday.is_empty() {
            let days: Vec<_> = self.by_weekday.iter().map(DayOfWeek::as_rrule).collect();
            parts.push(format!("BYDAY={}", days.join(",")));
        }

        if !self.by_month_day.is_empty() {
            let days: Vec<_> = self.by_month_day.iter().map(|d| d.to_string()).collect();
            parts.push(format!("BYMONTHDAY={}", days.join(",")));
        }

        match &self.limit {
            RecurrenceLimit::Count { count } => parts.push(format!("COUNT={count}")),
            RecurrenceLimit::Until { time } => {
                const UNTIL_FMT: &[BorrowedFormatItem<'static>] =
                    time::macros::format_description!("[year][month][day]T[hour][minute][second]Z");

                let dt = time.to_offset(time::UtcOffset::UTC).format(UNTIL_FMT)?;
                parts.push(format!("UNTIL={dt}"));
            }
            RecurrenceLimit::Infinite => {}
        }

        Ok(parts.join(";"))
    }
}
