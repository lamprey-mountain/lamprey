//! Journals can be configured with [`Config`]

/// Configuration for a [`super::Journal`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The maximum size of a single block in bytes.
    ///
    /// Defaults to 1 MiB and is capped at 16 MiB.
    pub max_block_len: u64,

    /// How many blocks to retain.
    ///
    /// `None` retains blocks forever. Truncating before a retained block will
    /// cause it to be dropped.
    pub max_block_count: Option<u64>,

    /// Whether this journal is read-only.
    ///
    /// Frozen journals can't be appended to or truncated, but can still be
    /// deleted.
    pub frozen: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_block_len: 1024 * 1024,
            max_block_count: None,
            frozen: false,
        }
    }
}
