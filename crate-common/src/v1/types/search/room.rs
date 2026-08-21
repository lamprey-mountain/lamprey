use lamprey_macros::record;

use crate::v1::types::{Room, RoomId, search::common::SearchRequest};

#[record]
pub struct RoomSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    /// what order to return results in
    #[serde(default)]
    pub order: RoomSearchOrderField,

    /// field to sort by
    #[serde(default)]
    pub sort_field: RoomSearchOrderField,
}

/// which field to order room search results by
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum RoomSearchOrderField {
    /// sort by number of members
    #[default]
    Members,

    /// sort by creation time
    Created,

    /// sort by room name
    Name,

    /// sort by room id
    Id,
}

#[record]
pub struct RoomSearch {
    /// the ids of the matched rooms
    pub results: Vec<RoomId>,

    /// the rooms
    pub rooms: Vec<Room>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}
