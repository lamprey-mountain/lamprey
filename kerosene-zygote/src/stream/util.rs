use core::fmt;

use bytes::Bytes;

/// a point in the stream
///
/// use [`Position`] when you need both units at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offset {
    /// an offset measured in blocks
    Block(u64),

    /// an offset measured in bytes
    Bytes(u64),
}

/// an amount of data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// a number of blocks
    Block(u64),

    /// a number of bytes
    Byte(u64),
}

/// a point in the stream, measured in both blocks and bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// the block offset
    pub block: u64,

    /// the byte offset
    pub byte: u64,
}

/// when to stop reading
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    /// read until a number of blocks/bytes have been read
    Count(Size),

    /// read until an offset is reached
    Until(Offset),

    /// keep reading forever
    ///
    /// when reading, this watches the stream. when truncating, this deletes all stream data.
    Forever,
}

/// a block of data read from the stream
#[derive(Clone)]
pub struct Block {
    /// the position of this block in the stream
    pub position: Position,

    /// the block's data
    // NOTE: maybe i should implement streaming incase a block's data is very large
    pub data: Bytes,
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Block")
            .field("position", &self.position)
            .field("data.len()", &self.data.len())
            .finish()
    }
}
