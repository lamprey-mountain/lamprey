pub mod backend;
pub mod client;
pub mod error;
pub mod mesh;
pub mod server;
pub mod util;

// TEMP: old code, will be removed soon
// mod backbone_old;
// mod peer_old;
// mod sfu_old;

pub use server::sfu::Sfu;

pub(crate) mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::util::{CallSlot, PeerSlot, SinkSlot, TrackSlot};

    pub use futures::{Sink, SinkExt, Stream, StreamExt};
    pub use std::sync::Arc;

    pub use str0m::channel::ChannelId as SChannelId;
    pub use str0m::media::{KeyframeRequestKind as SKeyframeRequestKind, Mid as SMid, Rid as SRid};
    pub use str0m::{Event as SEvent, Input as SInput, Output as SOutput};
}
