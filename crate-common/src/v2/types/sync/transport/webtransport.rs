use bytes::Bytes;
use lamprey_macros::record;

use crate::v2::types::sync::transport::{Compression, Encoding, Version};

/// query parameters when establishing a webtransport sync connection
#[record(params)]
pub struct Params {
    pub version: Version,

    pub compression: Option<Compression>,

    #[serde(default)]
    pub encoding: Encoding,
}

/// webtransport message framing
// NOTE: this struct is probably never going to get used - the client doesnt need this to write data, the server will probably use &[u8] for zero copy
pub struct Envelope {
    pub len: u32,
    pub data: Bytes,
}
