use lamprey_macros::record;

use crate::{
    v1::types::{MediaId, User, search::common::SearchRequest},
    v2::types::media::Media,
};

#[record]
pub struct MediaSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    #[serde(default)]
    pub sort_field: MediaSearchOrderField,
}

#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum MediaSearchOrderField {
    #[default]
    Created,
    Name,
    Id,
}

#[record]
pub struct MediaSearch {
    /// the ids of the matched media
    pub results: Vec<MediaId>,

    /// the media
    pub media: Vec<Media>,

    /// the media creators/uploaders
    pub users: Vec<User>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}
