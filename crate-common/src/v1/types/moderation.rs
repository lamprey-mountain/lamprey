#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::v1::types::{ChannelId, ReportId, UserId};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

/// moderation report
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Report {
    pub id: ReportId,

    /// the associated thread id of this report. every report has a thread created for it.
    pub thread_id: ChannelId,

    /// user id of who reported this
    // do i want to make this an Option and allow anonymous reports?
    pub reporter_id: UserId,

    /// built in reason
    pub reason: ReportReason,

    /// user supplied note
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 4096)
    )]
    pub note: Option<String>,

    /// where the report is being sent to
    pub destination: ReportDestination,
}

/// who the report is sent to
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ReportDestination {
    /// send to room moderators
    Room,

    /// send to server moderators
    Server,
}

// these reasons are more or less copied from revolt.chat for now
// (theres quite a lot considering ui design, it looks like they're meant to be nested inside subcategories?)
// i should probably reduce it down to maybe 4 to 8 preset reasons
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ReportReason {
    /// generally illegal
    Illegal,

    /// Selling or facilitating use of drugs or other illegal goods
    IllegalGoods,

    /// Extortion or blackmail
    IllegalExtortion,

    /// Revenge or child pornography
    IllegalPornography,

    /// Illegal hacking activity
    IllegalHacking,

    /// Extreme violence, gore, or animal cruelty With exception to violence portrayed in media / creative arts
    ExtremeViolence,

    /// Content that promotes harm to others / self
    PromotesHarm,

    /// Unsolicited advertisements
    // seems too similar to SpamAbuse?
    UnsolicitedSpam,

    /// This is a raid
    Raid,

    /// user/content: Spam or platform abuse
    SpamAbuse,

    /// user/content: Scams or fraud
    ScamsFraud,

    /// Distribution of malware or malicious links
    Malware,

    /// Harassment or abuse targeted at another user
    Harassment,

    /// user: profile breaks tos
    InappropriateProfile,

    /// user: impersonating someone else
    Impersonation,

    /// user: attempted to evade ban
    BanEvasion,

    /// user: not old enough
    Underage,

    /// something else; see note
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ReportCreate {
    /// built in reason
    pub reason: ReportReason,

    /// user supplied note
    // copy url of whatever is reported here
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 4096)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4096)))]
    pub note: Option<String>,
    // maybe copy thread create params here?
}
