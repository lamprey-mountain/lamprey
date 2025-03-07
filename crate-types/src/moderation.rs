use serde::{Deserialize, Serialize};

use crate::{MediaId, MessageId, ReportId, RoomId, ThreadId, UserId};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// moderation report
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Report {
    pub id: ReportId,
    pub creator_id: UserId,
    pub reason: ReportReason,
    pub destination: ReportDestination,
    pub target: ReportTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ReportDestination {
    /// send to room moderators
    Room,

    /// send to server moderators
    Server,
}

// might need to have some kind of Anything enum that can be anything in the api
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ReportTarget {
    User { target_id: UserId },
    Room { target_id: RoomId },
    Thread { target_id: ThreadId },
    Message { target_id: MessageId },
    Media { target_id: MediaId },
}

/// synced with thread if there is any
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ReportStatus {
    Open,
    Duplicate,
    Invalid,
    Resolved,
}

// these reasons are more or less copied from revolt.chat for now
// (theres quite a lot considering ui design, it looks like they're meant to be nested inside subcategories?)
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

    /// user specified
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ReportCreate {
    pub reason: ReportReason,
    pub destination: ReportDestination,
}

impl ReportStatus {
    pub fn is_active(&self) -> bool {
        *self == ReportStatus::Open
    }
}
