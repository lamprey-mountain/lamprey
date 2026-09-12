use lamprey_macros::record;

use crate::v1::types::{RoomMember, User, UserId};

#[record]
pub struct CalendarEventParticipant {
    pub user_id: UserId,
    pub status: CalendarRsvpStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<RoomMember>,
}

#[record]
#[derive(Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CalendarRsvpStatus {
    Interested,
    Uninterested,
}

#[record]
pub struct CalendarEventParticipantPut {
    pub status: CalendarRsvpStatus,
}
