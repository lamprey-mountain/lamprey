// TODO: impl and use this

use bytes::Bytes;
use flate2::{Compress, Decompress};

use super::Transport;

/// compression that can be applied to a transport
pub trait Compression {
    /// the type of the compressed transport
    type Transport;

    // TODO: finish this type
}

/// don't compress anything
#[derive(Default)]
pub struct Identity {
    /// prevent manually constructing this type
    _nope: (),
}

/// compress with deflate
pub struct Deflate {
    compressor: Compress,
    decompressor: Decompress,
    buffer: Bytes,
}

// TODO
// impl Compression for Identity {}
// impl Compression for Deflate {}

impl Default for Deflate {
    fn default() -> Self {
        Self {
            compressor: Compress::new(flate2::Compression::default(), true),
            decompressor: Decompress::new(true),
            buffer: Bytes::new(),
        }
    }
}

/// extension trait for Transport that allows compression
pub trait TransportExt: Transport {
    /// compress this transport
    fn compress<C: Compression>(self: Box<Self>, compression: C) -> C::Transport {
        todo!()
    }
}
