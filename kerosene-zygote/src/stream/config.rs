/// configuration for a node
pub struct GlobalConfig {
    pub storage: Storage,
    // TODO: design and implement
}

/// configuration for a stream
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// how long to retain data for
    pub retention: Retention,

    /// data compression config
    pub compression: Compression,

    /// whether this stream is read only
    ///
    /// Frozen streams can't be appended or truncated, but can still be deleted.
    pub frozen: bool,

    /// storage configuration
    pub storage: Storage, // NOTE: maybe make this only exist in GlobalConfig?

    /// durability configuration
    pub durability: Durability,
}

/// how durable data should be
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// prioritize speed
    ///
    /// The stream is never synced unless `sync()` is explicitly called.
    Lazy,

    /// balance speed and durability
    ///
    /// The stream is occasionally `sync()`ed automatically. `sync()` flushes to the local filesystem.
    Semidurable,

    /// prioritize durability
    ///
    /// The stream is occasionally `sync()`ed automatically. `sync()` flushes to the object directory.
    #[default]
    Durable,
}

/// how to shuffle data between memory, disk, and object storage
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Storage {
    /// the maximum number of bytes to store in memory
    pub memory_limit: usize,

    /// the maximum number of bytes to store in disk
    pub disk_limit: usize,

    /// the maximum number of bytes to store in object storage
    pub object_limit: usize,
}

/// how to compress data
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Compression {
    /// how to compress bytes on disk
    pub disk: CompressionType,

    /// how to compress bytes on object storage
    pub object: CompressionType,
}

/// how to compress data
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    #[default]
    None,
    Lz4,
    Zstd,
}

/// automatically truncate data at the beginning of the stream
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Retention {
    /// the maximum number of blocks to retain
    ///
    /// zero means no limit
    pub max_blocks: usize,

    /// the maximum number of bytes to retain
    ///
    /// zero means no limit
    pub max_bytes: usize,
}
