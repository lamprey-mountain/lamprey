use lamprey_macros::record;

use crate::v1::types::{User, UserId, search::common::SearchRequest};

#[record]
pub struct UserSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    #[serde(default)]
    pub sort_field: UserSearchOrderField,
}

#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum UserSearchOrderField {
    /// the user's name
    #[default]
    Name,

    /// the user's created_at
    Created,

    /// when the user was registered
    Registered,

    /// the user's id
    Id,
}

#[record]
pub struct UserSearch {
    /// the ids of the matched users
    pub results: Vec<UserId>,

    /// the users
    pub users: Vec<User>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}

// TODO: index these extra fields:
// puppet: bool,
// guest: bool,
// server_role_id: Vec<RoleId>,
// member_of_room_id: Vec<RoleId>,
