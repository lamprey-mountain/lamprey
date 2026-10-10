use lamprey_macros::record;

use crate::{util::registry::export_models, v1::types::misc::Color};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use super::util::Diff;

mod event;
mod instance;
mod participant;
mod recurrence;

pub use event::*;
pub use instance::*;
pub use participant::*;
pub use recurrence::*;

/// channel metadata for a calendar
#[record]
pub struct Calendar {
    /// the color of this calendar
    pub color: Option<Color>,

    /// the default timezone events in this calendar should be created in
    pub default_timezone: Timezone,
}

#[record]
#[derive(Diff)]
pub struct CalendarPatch {
    #[serde(default, deserialize_with = "some_option")]
    pub color: Option<Option<Color>>,
    pub default_timezone: Option<Timezone>,
}

/// a timezone
// TODO: validate? maybe allow only specific timezones?
#[record]
#[derive(PartialEq, Eq)]
pub struct Timezone(pub String);

export_models!(Calendar, CalendarPatch, Timezone, use participant);
