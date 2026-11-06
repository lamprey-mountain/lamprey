//! Durable streams via the [`Journal`] type.
//!
//! A journal is an append-only sequence of [`Block`]s. Journals are cheap to
//! create and are addressed by a random [`JournalId`]. Their contents are stored
//! in a backend.
//!
//! A [`Journal`] appends blocks and manages its lifecycle, while a [`Reader`]
//! reads blocks. Journals are cheap, cloneable handles, so many readers and
//! writers can share the same journal concurrently.

// TODO: configure compression per "segment". eg. newly written segments might be uncompressed or use lz4, older/larger merged segments may use zstd?
// TODO: configure durability on the backend instead of journal

mod backend;
mod config;
mod error;
mod journal;
mod meta;
mod reader;

pub(crate) use backend::Backend;
pub use backend::Metadata;
pub use backend::memory::MemoryBackend;
pub use config::Config;
pub use error::{OpenError, ReadError, WriteError};
pub use journal::{Block, Journal, JournalId};
pub use reader::Reader;
