//! The [`Journal`] handle and types it uses.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
};

use bytes::Bytes;

use super::{Backend, Config, Metadata, OpenError, ReadError, Reader, WriteError};

// TODO: impl Display, FromStr, serde
// TODO: make the u64 be private, add from/into u64
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JournalId(pub u64);

impl JournalId {
    /// The id of the meta journal.
    ///
    /// The meta journal stores journal [events](super::meta::JournalEvent) as
    /// blocks. Deleting it truncates its history, but the backend recreates it
    /// as soon as another event is appended.
    pub const META: Self = Self(0);

    /// Returns `true` if this is the meta journal.
    #[inline]
    pub fn is_meta(self) -> bool {
        self == Self::META
    }
}

/// A block of data written to or read from a journal.
#[derive(Debug, Clone)]
pub struct Block {
    /// The sequence number of this block.
    pub seq: u64,

    /// The block's data.
    pub data: Bytes,
}

/// A cheap, cloneable handle to a journal.
///
/// Cloning a journal creates another handle to the same journal, not another
/// journal. Appends from any handle are ordered by the backend.
#[derive(Clone)]
pub struct Journal {
    backend: Arc<dyn Backend>,
    id: JournalId,
    config: Config,
    pending: Arc<Mutex<VecDeque<Bytes>>>,
}

// TODO: add some of these methods to Reader?
// TODO: fn pending_size
// TODO: cache some stuff locally? so less methods are async. maybe make everything except fn flush/sync async?
// TODO: #[inline] some methods
// TODO: queue_write should still try to write eventually, it's just not required to be durable immediately
impl Journal {
    /// Get this journal's unique identifier.
    pub fn id(&self) -> JournalId {
        self.id
    }

    /// Get this journal's configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get the number of blocks that have been queued but not written yet.
    ///
    /// Queued blocks are written by [`sync`](Self::sync), [`write`](Self::write),
    /// or [`freeze`](Self::freeze).
    pub fn pending_len(&self) -> u64 {
        self.pending().len() as u64
    }

    /// Get how many bytes the retained blocks occupy on the backend.
    ///
    /// This may differ from [`size`](Self::size) when compression is enabled.
    pub async fn physical_size(&self) -> Result<u64, ReadError> {
        Ok(self.backend.stat(self.id).await?.physical_size)
    }

    /// Get the total length of this journal in bytes.
    ///
    /// This is the sum of all written block lengths.
    pub async fn size(&self) -> Result<u64, ReadError> {
        Ok(self.backend.stat(self.id).await?.size)
    }

    /// Queue a block to be written later.
    ///
    /// This returns immediately. The block is not sent to the backend until
    /// [`sync`](Self::sync), [`write`](Self::write), or
    /// [`freeze`](Self::freeze) is called.
    ///
    /// Fails with [`WriteError::Oversized`] if `block` is larger than
    /// [`Config::max_block_len`], and with [`WriteError::Frozen`] if this
    /// journal is frozen.
    pub fn queue_write(&self, block: Bytes) -> Result<(), WriteError> {
        self.check_write(&block)?;
        self.pending().push_back(block);
        Ok(())
    }

    /// Append a block to this journal.
    ///
    /// Queued blocks are flushed first. The append is atomic: the block is
    /// either fully written or not written at all. Returns the sequence number
    /// of the newly written block.
    ///
    /// Fails with [`WriteError::Oversized`] if `block` is larger than
    /// [`Config::max_block_len`], and with [`WriteError::Frozen`] if this
    /// journal is frozen.
    pub async fn write(&self, block: Bytes) -> Result<u64, WriteError> {
        self.check_write(&block)?;
        self.flush().await?;
        self.backend.append(self.id, block).await
    }

    /// Flush queued blocks and ensure they are durably persisted.
    ///
    /// What "durably persisted" means depends on the backend.
    pub async fn sync(&self) -> Result<(), WriteError> {
        self.flush().await?;
        self.backend.sync(self.id).await
    }

    /// Make this journal immutable.
    ///
    /// Flushes queued blocks first. Returns whether this journal was newly
    /// frozen. Freezing an already frozen journal is a no-op.
    pub async fn freeze(&self) -> Result<bool, WriteError> {
        self.flush().await?;
        self.backend.freeze(self.id).await
    }

    /// Delete this journal, along with all of its blocks.
    ///
    /// All other handles to this journal become invalid and must be reopened.
    /// Queued blocks are discarded.
    pub async fn delete(self) -> Result<(), WriteError> {
        self.backend.delete(self.id).await
    }

    /// Open a reader at `start`.
    ///
    /// The reader stops when it reaches the end of the journal.
    pub async fn read(&self, start: u64) -> Result<Reader, ReadError> {
        let end = self.backend.stat(self.id).await?.end;
        let stream = self.backend.read(self.id, start, false).await?;
        Ok(Reader::new(self.clone(), start, Some(end), false, stream))
    }

    /// Open a reader at the end of this journal.
    ///
    /// The reader follows new blocks as they are written.
    pub async fn tail(&self) -> Result<Reader, ReadError> {
        let start = self.backend.stat(self.id).await?.end;
        let stream = self.backend.read(self.id, start, true).await?;
        Ok(Reader::new(self.clone(), start, None, true, stream))
    }

    fn check_write(&self, block: &Bytes) -> Result<(), WriteError> {
        if self.config.frozen {
            return Err(WriteError::Frozen);
        }
        if block.len() as u64 > self.config.max_block_len {
            return Err(WriteError::Oversized);
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), WriteError> {
        loop {
            let block = { self.pending().pop_front() };
            let Some(block) = block else {
                return Ok(());
            };

            // keep the block so it can be requeued if the append fails
            if let Err(err) = self.backend.append(self.id, block.clone()).await {
                self.pending().push_front(block);
                return Err(err);
            }
        }
    }

    fn pending(&self) -> MutexGuard<'_, VecDeque<Bytes>> {
        self.pending.lock().expect("journal pending queue poisoned")
    }
}
