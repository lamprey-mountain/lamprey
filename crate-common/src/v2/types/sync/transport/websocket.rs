use bytes::Bytes;
use lamprey_macros::record;

use crate::v2::types::sync::transport::{Compression, Encoding, Version};

/// query parameters when establishing a websocket sync connection
#[record(params)]
pub struct Params {
    pub version: Version,

    pub compression: Option<Compression>,

    #[serde(default)]
    pub encoding: Encoding,
}

/// websocket message framing
pub enum Envelope {
    /// open a stream
    // NOTE: this is kinda redundant with Data?
    Open(u32),

    /// send data to a stream
    Data(u32, Bytes),

    /// close a stream
    Close(u32),
}
