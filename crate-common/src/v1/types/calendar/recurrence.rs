use std::collections::HashSet;

use lamprey_macros::record;

use crate::v1::types::{
    error::{ErrorField, ErrorFieldType},
    misc::Time,
};

#[record]
#[derive(PartialEq, Eq)]
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
    pub limit: RecurrenceLimit,

    /// repeat every n (days/weeks/months/years)
    pub interval: u32,
}

#[record]
#[derive(PartialEq, Eq)]
#[serde(tag = "type")]
pub enum RecurrenceLimit {
    /// repeat this event forever
    Infinite,

    /// repeat this event n times
    Count { count: u32 },

    /// repeat this event until this time
    Until { time: Time },
}

#[record]
#[derive(Copy, PartialEq, Eq)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// a day of the week
#[record]
#[derive(Copy, PartialEq, Eq, Hash)]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

// TODO: impl Validator for this
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

    /// convert to a rfc rrule string
    pub fn to_rrule(&self) -> String {
        let mut rrule = vec![];

        // TODO: extract out display/fromstr (use strum?)
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
                    // TODO: extract out display/fromstr (use strum?)
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

        match &self.limit {
            RecurrenceLimit::Count { count } => {
                rrule.push(format!("COUNT={}", count));
            }
            RecurrenceLimit::Until { time } => {
                let dt = time.to_offset(time::UtcOffset::UTC);
                // TODO: use new version when parsing
                let fmt = time::format_description::parse_borrowed::<1>(
                    "[year][month][day]T[hour][minute][second]Z",
                )
                .unwrap();
                rrule.push(format!("UNTIL={}", dt.format(&fmt).unwrap()));
            }
            RecurrenceLimit::Infinite => {}
        }

        rrule.join(";")
    }
}
