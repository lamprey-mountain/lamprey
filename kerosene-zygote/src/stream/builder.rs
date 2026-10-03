use crate::stream::Stream;

pub struct StreamBuilder {
    _nope: (), // prevent constructing manually
}

impl StreamBuilder {
    /// set a limit to how many blocks should be retained
    pub fn retain_blocks(self, n: u64) -> Self {
        todo!()
    }

    /// set a limit to how many bytes should be retained
    pub fn retain_bytes(self, n: u64) -> Self {
        todo!()
    }

    // TODO: add more config options to builder
    // fn durability
    // fn compress_disk
    // fn compress_object

    /// open a stream, failing if one doesnt exist
    pub fn open(self) -> Result<Stream, OpenError> {
        todo!()
    }

    /// open or create a stream
    pub fn create(self) -> Result<Stream, OpenError> {
        todo!()
    }

    /// create a stream, failing if one already exists
    pub fn create_new(self) -> Result<Stream, OpenError> {
        todo!()
    }
}

impl Stream {
    /// build a stream
    pub fn builder(name: &str) -> StreamBuilder {
        todo!()
    }
}

/// an error that occurred while opening a stream
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// this stream already exists
    #[error("this stream already exists")]
    Exists,

    /// this stream doesn't exist
    #[error("this stream doesn't exist")]
    Missing,

    /// configuration mismatch
    ///
    /// you tried to open a stream with a different config from what its using
    #[error("configuration mismatch")]
    ConfigMismatch,
}
