use std::time::Duration;

/// buffer size split between indexing threads
///
/// currently set to 100mb
pub const INDEXING_BUFFER_SIZE: usize = 100_000_000;

/// how frequently to commit the index
pub const COMMIT_INTERVAL: Duration = Duration::from_secs(5);

/// the maximum of uncommitted documents before needing to commit
pub const MAX_UNCOMMITTED: usize = 1000;

// TODO: finetune these numbers. maybe dynamically change them based on what's happening, eg. raise limits during bulk imports?
