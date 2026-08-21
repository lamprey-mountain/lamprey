use lamprey_macros::record;
use uuid::Uuid;

use crate::v1::types::search::common::SearchRequest;

#[record]
pub struct EverythingSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    #[serde(default)]
    pub sort_field: EverythingSearchOrderField,
}

#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum EverythingSearchOrderField {
    #[default]
    Id,
}

#[record]
pub struct EverythingSearch {
    pub results: Vec<Uuid>,
    pub items: Vec<EverythingSearchItem>,

    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}

#[record]
pub struct EverythingSearchItem {
    pub id: Uuid,

    // TODO: use Doctype here
    #[serde(rename = "type")]
    pub doctype: String,
}
