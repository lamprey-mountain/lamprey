use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Timelike};
use chrono_tz::Tz;
use common::v1::types::{
    calendar::{CalendarEvent, Timezone},
    util::Time,
};
use rrule::{RRule, RRuleSet, RRuleSetIter, Tz as RRuleTz, Unvalidated};
use time::OffsetDateTime;

use crate::{Error, Result, ServerStateInner};

pub struct ServiceCalendar {
    state: Arc<ServerStateInner>,
}

/// utility for calculating various things from recurrence rules
pub struct RecurrenceCalculator {
    calendar_event: CalendarEvent,
    rrule_set: RRuleSet,
}

/// utility for iterating through event instances
pub struct RecurrenceIterator(RRuleSetIter);

fn time_to_chrono(t: Time, tz: RRuleTz) -> DateTime<RRuleTz> {
    DateTime::from_timestamp(t.unix_timestamp(), t.nanosecond())
        .unwrap()
        .with_timezone(&tz)
}

fn chrono_to_time(t: DateTime<RRuleTz>) -> Time {
    let offset_dt = OffsetDateTime::from_unix_timestamp(t.timestamp())
        .unwrap()
        .replace_nanosecond(t.nanosecond())
        .unwrap();
    Time::from(offset_dt)
}

impl RecurrenceCalculator {
    /// if this event ends, gets the last day this series ends on
    pub fn series_ends_at(&self) -> Option<Time> {
        let last_occurrence = self.rrule_set.clone().into_iter().last()?;

        let offset_dt = OffsetDateTime::from_unix_timestamp(last_occurrence.timestamp())
            .unwrap()
            .replace_nanosecond(last_occurrence.nanosecond())
            .unwrap();
        let last_time = Time::from(offset_dt);

        // Add the duration to get the end time of the last occurrence
        if let Some(duration) = self.duration() {
            Some(last_time + duration)
        } else {
            Some(last_time)
        }
    }

    /// if this event ends, gets the number of events in this series
    pub fn series_count(&self) -> usize {
        self.rrule_set.clone().into_iter().count()
    }

    /// calculate the default start date/time of the nth event
    pub fn nth_event_starts_at(&self, seq: usize) -> Option<Time> {
        if let Some(a) = self.rrule_set.clone().into_iter().skip(seq).next() {
            let t = OffsetDateTime::from_unix_timestamp(a.timestamp())
                .unwrap()
                .replace_nanosecond(a.nanosecond())
                .unwrap();
            Some(t.into())
        } else {
            None
        }
    }

    /// calculate the default end date/time of the nth event
    pub fn nth_event_ends_at(&self, seq: usize) -> Option<Time> {
        self.nth_event_starts_at(seq)
            .map(|t| t + self.duration().unwrap_or_default())
    }

    /// calculate the duration of this event
    pub fn duration(&self) -> Option<Duration> {
        let starts_at = self.calendar_event.starts_at;
        let ends_at = self.calendar_event.ends_at?;
        let d = *ends_at - *starts_at;
        Some(Duration::new(
            d.whole_seconds() as u64,
            d.subsec_nanoseconds() as u32,
        ))
    }

    /// get the chrono timezone of this event
    pub fn chrono_tz(&self) -> Tz {
        self.calendar_event
            .timezone
            .as_ref()
            .unwrap_or(&Timezone("UTC".to_string()))
            .0
            .parse()
            .unwrap()
    }

    /// get the rule timezone of this event
    pub fn rrule_tz(&self) -> RRuleTz {
        RRuleTz::Tz(self.chrono_tz())
    }

    pub fn iter(&self, before: Option<Time>, after: Option<Time>) -> RecurrenceIterator {
        let mut rrs = self.rrule_set.clone();

        if let Some(before_time) = before {
            let tz = self.rrule_tz();
            let before_dt = time_to_chrono(before_time, tz);
            rrs = rrs.before(before_dt);
        }

        if let Some(after_time) = after {
            let tz = self.rrule_tz();
            let after_dt = time_to_chrono(after_time, tz);
            rrs = rrs.after(after_dt);
        }

        RecurrenceIterator(rrs.into_iter())
    }
}

impl Iterator for RecurrenceIterator {
    type Item = Time;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(datetime_tz) = self.0.next() {
            Some(chrono_to_time(datetime_tz))
            // let offset_dt = OffsetDateTime::from_unix_timestamp(datetime_tz.timestamp())
            //     .unwrap()
            //     .replace_nanosecond(datetime_tz.nanosecond())
            //     .unwrap();
            // Some(Time::from(offset_dt))
        } else {
            None
        }
    }
}

impl ServiceCalendar {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self { state }
    }

    pub fn recurrence(calendar_event: CalendarEvent) -> Result<RecurrenceCalculator> {
        let Some(recurrence) = &calendar_event.recurrence else {
            return Err(Error::BadStatic("calendar event has no recurrence rule"));
        };

        let tz: Tz = calendar_event
            .timezone
            .as_ref()
            .unwrap()
            .0
            .parse()
            .map_err(|_| Error::BadStatic("invalid timezone"))?;
        let tz = RRuleTz::Tz(tz);
        let starts_at = time_to_chrono(calendar_event.starts_at, tz);

        let rrule: RRule<Unvalidated> = recurrence
            .to_rrule()
            .parse()
            .expect("to_rrule should only give valid rrules");
        let rrule = rrule.validate(starts_at).unwrap();
        let rrule_set = RRuleSet::new(starts_at).rrule(rrule).limit();
        Ok(RecurrenceCalculator {
            calendar_event,
            rrule_set,
        })
    }
}
