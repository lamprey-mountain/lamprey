pub mod peer;
pub mod sfu;
pub mod util;

use common::v1::types::{ChannelId, UserId};

pub type PeerId = (ChannelId, UserId);
