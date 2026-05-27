pub mod backbone;
pub mod backend;
pub mod error;
pub mod peer;
pub mod sfu;
pub mod signalling;
pub mod util;

pub use error::Error;

use common::v1::types::{ChannelId, UserId};

pub type PeerId = (ChannelId, UserId);
