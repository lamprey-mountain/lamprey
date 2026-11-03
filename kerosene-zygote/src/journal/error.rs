/// An error that occurred while opening a journal.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// this journal doesn't exist
    #[error("this journal doesn't exist")]
    Missing,

    // NOTE: probably can't exist anymore?
    // this is possible with name-based journals (s3-style), but probably not with id-based journals
    /// this journal already exists
    #[error("this journal already exists")]
    Exists,

    /// the backend failed
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// an error that occurred while reading a journal
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// this journal has been deleted
    ///
    /// if this journal has been recreated, reopen it to begin reading again.
    #[error("this journal has been deleted")]
    Deleted,

    /// the backend failed
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// an error that occurred while writing to a journal
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// this journal is frozen
    ///
    /// can occur while writing, syncing
    #[error("this journal is frozen")]
    Frozen,

    /// this journal has been deleted
    ///
    /// even if this journal is deleted and recreated with the exact same
    /// options, it must be reopened to begin writing again.
    ///
    /// can occur while writing, syncing, deleting, freezing
    #[error("this journal has been deleted")]
    Deleted,

    /// ran out of space
    ///
    /// can occur while writing
    #[error("ran out of space")]
    Oversized,

    /// the backend failed
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
