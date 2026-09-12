use lamprey_macros::record;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{CalendarEventId, misc::Color, pagination::PaginationDirection};

#[cfg(feature = "serde")]
use crate::v1::types::util::some_option;

use super::util::{Diff, Time};

mod event;
mod overwrite;
mod participant;
mod recurrence;

pub use event::*;
pub use overwrite::*;
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

/// a timezone
// TODO: validate? maybe allow only specific timezones?
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Timezone(pub String);

#[record]
#[derive(Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct CalendarEventListQuery {
    #[validate(range(max = 1024))]
    pub limit: Option<u16>,
    pub from: Option<CalendarEventId>,
    pub to: Option<CalendarEventId>,
    pub dir: Option<PaginationDirection>,
    pub from_time: Option<Time>,
    pub to_time: Option<Time>,
}

#[record]
#[derive(Default)]
#[cfg_attr(feature = "utoipa", derive(utoipa::IntoParams))]
pub struct CalendarEventParticipantQuery {
    /// whether to include user and member
    #[serde(default)]
    pub include_member: bool,
}

#[record]
#[derive(Diff)]
pub struct CalendarPatch {
    #[serde(default, deserialize_with = "some_option")]
    pub color: Option<Option<Color>>,
    pub default_timezone: Option<Timezone>,
}
