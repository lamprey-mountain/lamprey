use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{CalendarEventId, ChannelId, UserId};

use super::util::Time;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarEvent {
    pub id: CalendarEventId,
    pub channel_id: ChannelId,
    pub creator_id: Option<UserId>,
    #[cfg_attr(feature = "utoipa", schema(max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(max = 64)))]
    pub title: String,
    #[cfg_attr(feature = "utoipa", schema(max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    pub description: Option<String>,
    #[cfg_attr(feature = "utoipa", schema(max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(max = 512)))]
    pub location: Option<String>,
    pub url: Option<Url>,
    pub timezone: Option<String>,
    pub recurrence: Vec<Recurrence>,
    pub start: Time,
    pub end: Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarEventCreate {
    #[cfg_attr(feature = "utoipa", schema(max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(max = 64)))]
    pub title: String,
    #[cfg_attr(feature = "utoipa", schema(max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    pub description: Option<String>,
    #[cfg_attr(feature = "utoipa", schema(max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(max = 512)))]
    pub location: Option<String>,
    pub url: Option<Url>,
    pub timezone: Option<String>,
    pub recurrence: Vec<Recurrence>,
    pub start: Time,
    pub end: Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct CalendarEventPatch {
    #[cfg_attr(feature = "utoipa", schema(max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(max = 64)))]
    pub title: Option<String>,
    #[cfg_attr(feature = "utoipa", schema(max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(max = 4096)))]
    pub description: Option<Option<String>>,
    #[cfg_attr(feature = "utoipa", schema(max_length = 512))]
    #[cfg_attr(feature = "validator", validate(length(max = 512)))]
    pub location: Option<Option<String>>,
    pub url: Option<Option<Url>>,
    pub channel_id: Option<ChannelId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Cron(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum Recurrence {
    /// rrule
    Rule {
        cron: Cron,
        until: Option<Time>,
        count: Option<u32>,
    },

    /// rdate
    Include(Vec<Time>),

    /// exdate
    Exclude(Vec<Time>),
}
