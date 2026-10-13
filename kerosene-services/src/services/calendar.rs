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
}
