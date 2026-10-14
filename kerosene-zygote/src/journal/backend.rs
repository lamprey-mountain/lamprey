//! Journal data is stored in [`Backend`]s

/// A storage backend to store [`super::Journal`] data in.
pub trait Backend: Send + Sync {
    // TODO: design this trait
    // make fns use std::io::Error as the error type?
}

pub mod memory {
    //! A [`MemoryBackend`] stores data in memory

    //! A [`super::Backend`] that stores data in memory
    pub struct MemoryBackend {
        // ...
    }
    // TODO: impl MemoryBackend
}

pub mod disk {
    // TODO: DiskBackend
    // probably use tokio
}

pub mod distributed {
    // TODO: use raft/paxos for a write ahead log, store compacted segments in s3 (see cluster)
    // TODO: DistributedBackend
}
