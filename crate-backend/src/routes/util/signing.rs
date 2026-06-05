// TEMP
pub use common::v1::types::headers::{
    HEADER_ORIGIN, HEADER_PUBKEY, HEADER_SIGNATURE, HEADER_TIMESTAMP,
};

/// standard http header: the target host of this request
pub const HEADER_HOST: &str = "host";

pub use common::v1::types::federation::signing::{IncomingRequest, OutgoingRequest};
