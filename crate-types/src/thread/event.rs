#![allow(unused)]

use serde::{Deserialize, Serialize};

use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// #[cfg(feature = "validator")]
// use validator::Validate;

use crate::{misc::Color, text::OwnedText, util::Time, Media, ThreadId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ThreadInfoEventLocation {
    Geo(crate::media::Location),
    Url(url::Url),
}

// probably need a better repr
// doesn't need to be fully vanilla cron, can be more typesafe/user friendly if needed
// or use lib.rs/cron
// also figure out how i18n works for other calendar systems
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CronStr(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Cron {
    pub minutes: Vec<CronValue<60>>,
    pub hours: Vec<CronValue<60>>,
    pub days: Vec<CronValue<31>>,
    pub months: Vec<CronValue<12>>,
    pub year: Vec<CronValue<12>>,
    pub days_of_week: Vec<CronValue<7>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum CronValue<const MAX: u8> {
    All,
    Single(u8),
    Range(u8, u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum TimeGranularity {
    Day,
    Hour,
    Minute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum EventDuration {
    AllDay,
    Minutes(u64),
}

// could be part of ThreadState? unsure how to do this appropriately though
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum EventStatus {
    Scheduled,
    Active,
    // maybe these two are the same as archived
    Finished,
    Cancelled {
        // special case of Archived? maybe have something like is_cancelled for archived?
        // might be good to be able to have something like github's "closed as not planned"
        // alternatively, i could use special purpose tags
        cancelled_reason: OwnedText,
        cancelled_at: Time,
        cancelled_by: UserId,
    },
}

// are events a type of thread or their own thing? what if i want to tag events? maybe there might be one logical (repeating) event that creates multiple threads?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Event {
    // pub id: EventId,
    pub name: OwnedText,
    pub description: Option<OwnedText>,
    pub color: Option<Color>,
    pub icon: Option<Media>,
    pub banner: Option<Media>,

    pub location: ThreadInfoEventLocation,
    pub url: Url,

    // NOTE: time apparently stores timezone
    pub time: Time,
    pub time_granularity: TimeGranularity,
    pub repeats: Cron,
    pub until: Time,
    pub duration: EventDuration,

    pub user_limit: Option<u64>,
    pub user_rsvp_yes: u64,
    pub user_rsvp_no: u64,
    pub user_rsvp_maybe: u64,
    pub user_rsvp_invited: u64,
    pub user_rsvp_waitlisted: u64,
    pub autofill_waitlist: bool,
    pub status: EventStatus,
}

// maybe it could be an extension of ThreadMembership? eh maybe not
pub enum EventRsvpType {
    Yes,
    No,
    Maybe,
    Invited,
    Waitlisted,
}

pub struct EventRsvp {
    pub thread_id: ThreadId,
    pub user_id: UserId,
    pub status: EventRsvpType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeEventPublic {
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeEventPrivate {
    pub self_status: Option<EventRsvpType>,
    pub mention_count: u64,
}
