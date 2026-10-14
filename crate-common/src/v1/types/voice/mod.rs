//! types for the voice protocol
//!
//! Voice connections typically use webrtc. Each connection is represented with
//! a [`VoiceState`]. Voice states exist in channels of specific types: voice,
//! broadcast, dm, and gdm.

// TODO: add more doc comments

pub mod call;
pub mod channel;
pub mod datachannel;
pub mod error;
pub mod internal;
pub mod messages;
pub mod ring;
pub mod router;
pub mod rtc;
pub mod sfu;
pub mod track;
pub mod voice_state;

#[cfg(feature = "str0m")]
mod str0m;

pub use call::*;
pub use channel::*;
pub use error::*;
pub use ring::*;
pub use rtc::*;
pub use sfu::*;
pub use track::*;
pub use voice_state::{
    VoiceStateFull, VoiceStateMove, VoiceStateMoveBulk, VoiceStateParams, VoiceStateScreenshare,
    VoiceStateScreenshareUpdate,
};

// TODO: better names for VoiceState{Patch,Update}?

#[doc(inline)]
pub use voice_state::{VoiceState, VoiceStatePatch, VoiceStateUpdate};
