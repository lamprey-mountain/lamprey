// TODO: impl and use

use bytes::Bytes;

// NOTE: maybe instead of the planned sharding system, allow clients to open multiple websockets (with each connection supporting multiple streams)?

/// message framing
pub struct Message {
    pub stream: u64,
    pub op: Operation,
}

pub enum Operation {
    /// open this stream
    // NOTE: this is kinda redundant with Data?
    Open,

    /// send data to this stream
    Data(Bytes),

    /// close this stream
    Close,
}
