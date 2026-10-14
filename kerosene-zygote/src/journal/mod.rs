//! Durable streams via the [`Journal`] type.
//!
//! A journal is an append-only sequence of [`Block`]s. Journals are cheap to
//! create and are addressed by a random [`JournalId`]. Their contents are stored
//! in a [`Backend`].
//!
//! A [`Writer`] appends blocks to a journal while a [`Reader`] reads blocks.
//! Journals are a cheap, cloneable handles, so many readers and writers can share
//! the same journal concurrently.

// TODO: configure compression per "segment". eg. newly written segments might be uncompressed or use lz4, older/larger merged segments may use zstd?
// TODO: configure durability on the backend instead of journal
// TODO: have a special meta journals that logs journal creation/deletion. (how does this work if multiple backends are used? one meta journal per backend?)
// TODO; where do i add fn create, fn open to create Journals?

use std::sync::Arc;

use uuid::Uuid;

pub mod backend;
pub mod config;
pub mod error;

pub use backend::Backend;
pub use config::Config;
pub use error::{ReadError, WriteError};

// TODO: impl this
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JournalId(pub Uuid);

/// A handle to a journal
#[derive(Clone)]
pub struct Journal {
    _backend: Arc<dyn Backend>,
}

// TODO: fn len, size, impl AsyncRead
pub struct Reader;

// TODO: fn seq (get the sequence number of this block), fn close, impl AsyncWrite
pub struct Writer;

impl Journal {
    /// Get this journal's unique identifier.
    pub fn id(&self) -> JournalId {
        todo!()
    }

    /// Get this journal's configuration.
    pub fn config(&self) -> &Config {
        todo!()
    }

    /// Get the start of this journal.
    ///
    /// Returns the sequence number of the first block that is still retained.
    /// If this journal was never truncated, this is `0`.
    pub fn start(&self) -> u64 {
        todo!()
    }

    /// Get the end of this journal.
    ///
    /// Returns one past the sequence number of the last block, so that a full
    /// range is `start()..end()`. If the journal is empty, this is the same as
    /// [`start`](Self::start).
    pub fn end(&self) -> u64 {
        todo!()
    }

    /// Get the length of this journal.
    ///
    /// Returns the number of (non-truncated) blocks.
    #[inline]
    pub fn len(&self) -> u64 {
        self.end() - self.start()
    }

    /// Returns `true` if this journal has no blocks.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the on-disk filesize of this journal in bytes.
    ///
    /// This is how many bytes the retained blocks occupy on the backend, which
    /// may differ from the sum of [`Block::data`] lengths when compression is
    /// enabled.
    pub fn physical_size(&self) -> u64 {
        todo!()
    }

    /// Get the total length of this journal in bytes.
    ///
    /// This is effectively the sum of all block lengths.
    pub fn size(&self) -> u64 {
        todo!()
    }

    // TODO: fn read/tail
    // TODO: fn freeze, sync, truncate(_before), write -> Writer (these return result, journal may be read only (different from frozen))
}
