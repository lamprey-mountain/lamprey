use lamprey_macros::record;

use crate::v1::types::{AuditLogEntry, AuditLogEntryId, search::common::SearchRequest};

#[record]
pub struct AuditLogSearchRequest {
    #[serde(flatten)]
    pub inner: SearchRequest,

    #[serde(default)]
    pub sort_field: AuditLogSearchOrderField,
}

#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum AuditLogSearchOrderField {
    #[default]
    Created,
}

#[record]
pub struct AuditLogSearch {
    /// the ids of the matched audit log entries
    pub results: Vec<AuditLogEntryId>,

    /// the audit log entries
    pub entries: Vec<AuditLogEntry>,

    // TODO: copy AuditLogPaginationResponse here
    /// whether there are more threads
    pub has_more: bool,

    /// approximate count of total results that match this query
    pub total: u64,

    /// current page cursor
    pub cursor: Option<String>,
}
