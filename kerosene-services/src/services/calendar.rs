use std::time::Duration;

use chrono::{DateTime, Timelike};
use chrono_tz::Tz;
use common::v1::types::{
    calendar::{Calendar, CalendarEvent, Timezone},
    util::Time,
};
use rrule::{RRule, RRuleSet, RRuleSetIter, Tz as RRuleTz, Unvalidated};
use time::OffsetDateTime;

use crate::prelude::*;

mod recurrence;

pub use recurrence::{RecurrenceCalculator, RecurrenceIterator};

pub struct ServiceCalendar {
    globals: Globals,
}

impl ServiceCalendar {
    pub fn new(globals: Globals) -> Self {
        Self { globals }
    }

    pub fn recurrence<'a>(
        &self,
        cal: &Calendar,
        ce: &'a CalendarEvent,
    ) -> Result<RecurrenceCalculator<'a>> {
        Ok(RecurrenceCalculator::new(cal, ce))
    }
}
