//! utilities for storing document changes cold storage (ie. s3)

// TODO: implement and use this

// see crate-backend-core/src/types/documents.rs

// TODO: add paths to MediaPaths

// upload to s3://document/{document_id}/{branch_id}/{n}? how can i deduplicate changes?
pub struct CompactChangeset {
    // changes_pointer: Vec<CompactChange>,
    // changes_data: Vec<u8>,
}

// pub struct CompactChange {
//     off: u64,
//     len: u64,
// }
