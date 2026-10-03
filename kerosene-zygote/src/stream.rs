//! durable streams

use std::{future::Future, pin::Pin, task};

use bytes::Bytes;

use crate::stream::config::Config;

pub mod builder;
pub mod config;
pub mod util;

pub use util::{Block, Limit, Offset, Position, Size};

/// a handle to a stream
///
/// all operations, from opening to reading to writing, are atomic.
pub struct Stream {
    // ...
}

// TEMP: testing some stuff
// mod impl_mem;

// TODO: other impls
// mod impl_fs;
// mod impl_obj;

/// a readable stream
pub trait StreamRead {
    /// get this stream's name
    fn name(&self) -> &str;

    /// get this stream's config
    fn config(&self) -> &Config;

    /// get the start of the stream
    ///
    /// Returns the first position still retained. If this stream was never
    /// truncated, this is 0.
    fn start(&self) -> Position;

    /// get the end of the stream
    ///
    /// Returns the last position in the stream. If this stream was never
    /// truncated, this is the same as `len()`.
    fn end(&self) -> Position;

    /// read some data from the stream
    ///
    /// Starts reading from offset (inclusive), stops reading at limit
    /// (exclusive). You can start and stop reading mid-block.
    fn read(&self, offset: Offset, limit: Limit) -> Result<Reader, ReadError>;
}

/// an extension trait to add helpful methods to [`StreamRead`]
pub trait StreamReadExt: StreamRead {
    /// get the length of the stream
    #[inline]
    fn len(&self) -> Position {
        let start = self.start();
        let end = self.end();
        Position {
            block: end.block - start.block,
            byte: end.byte - start.byte,
        }
    }

    /// get the length of the stream in bytes
    #[inline]
    fn len_bytes(&self) -> u64 {
        self.len().byte
    }

    /// get the length of the stream in blocks
    #[inline]
    fn len_blocks(&self) -> u64 {
        self.len().block
    }

    /// get the start of the stream in bytes
    #[inline]
    fn start_bytes(&self) -> u64 {
        self.start().byte
    }

    /// get the start of the stream in blocks
    #[inline]
    fn start_blocks(&self) -> u64 {
        self.start().block
    }

    /// get the end of the stream in bytes
    #[inline]
    fn end_bytes(&self) -> u64 {
        self.end().byte
    }

    /// get the end of the stream in blocks
    #[inline]
    fn end_blocks(&self) -> u64 {
        self.end().block
    }

    /// tail the stream
    ///
    /// this is a shortcut to start watching a stream
    #[inline]
    fn tail(&self, offset: Offset) -> Result<Reader, ReadError> {
        self.read(offset, Limit::Forever)
    }
}

impl<R: ?Sized + StreamRead> StreamReadExt for R {}

pub struct Reader {
    _nope: (), // prevent constructing manually
}

impl Reader {
    /// get how much data will be read
    pub fn len(&self) -> Position {
        todo!()
    }

    /// get how many bytes will be read
    #[inline]
    pub fn len_bytes(&self) -> u64 {
        self.len().byte
    }

    /// get how many blocks will be read
    #[inline]
    pub fn len_blocks(&self) -> u64 {
        self.len().block
    }
}

impl futures::Stream for Reader {
    type Item = Block;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        todo!()
    }
}

/// a writable stream
///
/// each stream may have at most one writer at a time
pub trait StreamWrite: StreamRead {
    /// make this stream immutable
    ///
    /// Returns whether the stream is newly frozen. Freezing an already frozen
    /// stream is a no-op.
    fn freeze(&self) -> impl Future<Output = Result<bool, WriteError>> + Send;

    /// write a new block to the stream
    ///
    /// Returns the location of the newly written block.
    fn write(&self, data: Bytes) -> impl Future<Output = Result<Position, WriteError>> + Send;

    /// ensure this stream is durably persisted
    ///
    /// what "durably persisted" means depends on the stream config
    fn sync(&self) -> impl Future<Output = Result<(), WriteError>> + Send;

    /// prune unneeded data from this stream
    ///
    /// Deletes all data before `offset`. Writers will be able to continue
    /// writing without needing to reopen the stream.
    fn truncate_before(
        &self,
        offset: Offset,
    ) -> impl Future<Output = Result<(), WriteError>> + Send;

    /// delete this stream
    fn delete(self) -> impl Future<Output = Result<(), WriteError>> + Send;
}

pub trait StreamWriteExt: StreamWrite {
    /// delete **all data** from this stream
    fn truncate(&self) -> impl Future<Output = Result<(), WriteError>> + Send {
        self.truncate_before(Offset::Bytes(self.len().byte))
    }
}

impl<W: ?Sized + StreamWrite> StreamWriteExt for W {}

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
