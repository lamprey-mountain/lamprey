//! Journal data is stored in [`Backend`]s.

use std::{future::Future, pin::Pin};

use bytes::Bytes;
use futures::stream::BoxStream;

use super::{Block, Config, JournalId, OpenError, ReadError, WriteError};

pub mod disk;
pub mod distributed;
pub mod memory;

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A stream of blocks read from a journal.
pub(crate) type BlockStream = BoxStream<'static, Result<Block, ReadError>>;

/// Metadata about a journal.
#[derive(Debug, Clone)]
pub struct Metadata {
    pub config: Config,

    /// One past the last block written to this journal.
    pub end: u64,

    /// The sum of all block lengths.
    pub size: u64,

    /// How many bytes the journal occupies on the backend.
    pub physical_size: u64,

    pub frozen: bool,
}

/// A storage backend for journals.
///
/// Implementations must reserve [`JournalId::META`] for the journal catalog.
/// Creating, freezing, and deleting a journal appends a
/// [`super::meta::JournalEvent`] block to it. If the meta journal is deleted,
/// the backend must lazily recreate it the next time an event is appended.
///
/// The meta journal is per backend. Each backend has its own catalog.
pub trait Backend: Send + Sync + 'static {
    /// Create a new journal and return its id and metadata.
    fn create(&self, config: Config) -> BoxFuture<'_, Result<(JournalId, Metadata), OpenError>>;

    /// Open an existing journal.
    fn open(&self, id: JournalId) -> BoxFuture<'_, Result<Metadata, OpenError>>;

    /// Delete a journal and all of its blocks.
    fn delete(&self, id: JournalId) -> BoxFuture<'_, Result<(), WriteError>>;

    /// List all journals in this backend.
    fn list(&self) -> BoxStream<'static, Result<(JournalId, Metadata), ReadError>>;

    /// Append a block and return its sequence number.
    fn append(&self, id: JournalId, block: Bytes) -> BoxFuture<'_, Result<u64, WriteError>>;

    /// Flush blocks written so far according to this backend's durability.
    fn sync(&self, id: JournalId) -> BoxFuture<'_, Result<(), WriteError>>;

    /// Freeze a journal. Returns whether it was newly frozen.
    fn freeze(&self, id: JournalId) -> BoxFuture<'_, Result<bool, WriteError>>;

    /// Get a journal's current metadata.
    fn stat(&self, id: JournalId) -> BoxFuture<'_, Result<Metadata, ReadError>>;

    /// Read blocks starting at `start`.
    ///
    /// If `tail` is `false`, the stream ends at the journal's end at the time
    /// the read started. If `tail` is `true`, it waits for and yields new
    /// blocks as they are appended.
    fn read(
        &self,
        id: JournalId,
        start: u64,
        tail: bool,
    ) -> BoxFuture<'_, Result<BlockStream, ReadError>>;
}
