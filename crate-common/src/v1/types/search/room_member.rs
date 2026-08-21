use crate::v1::types::search::common::SearchRequest;
use lamprey_macros::record;

#[record]
pub struct RoomMemberSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    /// field to sort by
    #[serde(default)]
    pub sort_field: RoomMemberSearchOrderField,
}

#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum RoomMemberSearchOrderField {
    /// sort by time user joined the room
    #[default]
    Joined,

    /// sort by user id
    UserId,
}
