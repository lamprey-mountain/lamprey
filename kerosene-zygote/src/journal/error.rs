/// an error that occurred while reading a stream
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// the data at the requested start position was truncated
    #[error("the data at the requested start position was truncated")]
    Truncated,

    /// this stream has been deleted
    ///
    /// if this stream has been recreated, reopen the stream to begin reading again.
    #[error("this stream has been deleted")]
    Deleted,
}

/// an error that occurred while writing to a stream
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// this stream is frozen
    ///
    /// can occur while writing, syncing, truncating
    #[error("this stream is frozen")]
    Frozen,

    /// this stream has been deleted
    ///
    /// even if this stream is deleted and recreated with the exact same
    /// options, the stream must be reopened to begin writing again.
    ///
    /// can occur while writing, syncing, truncating, deleting, freezing
    #[error("this stream has been deleted")]
    Deleted,

    /// ran out of space
    ///
    /// can occur while writing
    #[error("ran out of space")]
    Oversized,
}
