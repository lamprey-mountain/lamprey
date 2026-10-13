//! various random utilities and misc types

use futures::Stream;

use crate::voice::{VoiceError, player::rtc::Packet};

mod create_source;
mod media_kind;

pub use create_source::CreateSource;
pub use media_kind::{Audio, MediaKind, Video};

/// a source that packets can be streamed from
pub trait MediaSource<M: MediaKind>: Send + Sync {
    // PERF: maybe box VoiceError if it gets too large
    fn stream<'a>(&'a mut self) -> impl Stream<Item = Result<Packet<M>, VoiceError>> + Send + 'a;

    // TODO: add
    // maybe allow accessing these through some sort of handle?
    // /// get the duration of this media source (if known)
    // fn duration(&self) -> Option<Duration>;
    //
    // /// get the position of this media source (if known)
    // fn position(&self) -> Option<Duration>; // NOTE: this should always be known? or if not, i could add a new node to calculate the current position
}
