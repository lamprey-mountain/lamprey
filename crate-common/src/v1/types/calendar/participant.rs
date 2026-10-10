use lamprey_macros::record;

use crate::{
    util::registry::export_models,
    v1::types::{RoomMember, User, UserId},
};

/// the status of a participant
#[record]
#[derive(Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CalendarRsvpStatus {
    /// invited to this event
    ///
    /// can be set by the event creator and users with `CalendarEventManage`
    Invited,

    // these can only be set by the target user
    Accepted,
    Tentative,
    Declined,
}

#[record]
#[derive(Default)]
pub struct CalendarParticipantCounts {
    /// the number of users who have been invited
    pub invited: u64,

    /// the number of users who have accepted
    pub accepted: u64,

    /// the number of users who are tentative
    pub tentative: u64,

    /// the number of users who have declined
    pub declined: u64,
}

#[record]
pub struct CalendarParticipant {
    pub user_id: UserId,
    pub status: CalendarRsvpStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<RoomMember>,
}

#[record]
pub struct CalendarParticipantPut {
    pub status: CalendarRsvpStatus,
}

#[record]
pub struct CalendarParticipantInvite {
    #[schema(required = false, max_length = 256)]
    #[validate(length(max = 256))]
    pub user_ids: Vec<UserId>,
}

#[record(params)]
#[derive(Default)]
pub struct CalendarParticipantQuery {
    /// whether to include user and member
    #[serde(default)]
    pub include_member: bool,
}

impl CalendarParticipantCounts {
    pub fn total(&self) -> u64 {
        self.invited + self.accepted + self.tentative + self.declined
    }

    pub fn responded(&self) -> u64 {
        self.accepted + self.tentative + self.declined
    }
}

impl CalendarRsvpStatus {
    #[cfg(any())]
    // TODO: use or remove
    fn as_partstat(&self) -> &'static str {
        match self {
            Self::Invited => "NEEDS-ACTION",
            Self::Accepted => "ACCEPTED",
            Self::Tentative => "TENTATIVE",
            Self::Declined => "DECLINED",
        }
    }
}

export_models!(
    CalendarParticipant,
    CalendarRsvpStatus,
    CalendarParticipantPut,
    CalendarParticipantQuery
);
