// use crate::prelude::*;
use common::v1::types::voice::{VoiceState, internal::SfuPermissions};
use slotmap::new_key_type;

use crate::util::permissions::Permissions;

pub mod permissions;
pub mod simulcast;
pub mod stun;

new_key_type! {
    /// slotmap key for a webrtc peer
    pub struct PeerSlot;

    /// slotmap key for a track
    ///
    /// mids are local to each peer, `TrackId`s are shared
    pub struct TrackSlot;

    // TODO: split apart TrackSlot
    // pub struct InboundSlot;
    // pub struct OutboundSlot;

    pub struct CallSlot;
}

/// a voice state with extra info, for the server
pub struct SfuVoiceState {
    pub inner: VoiceState,
    pub permissions: SfuPermissions,
}

impl SfuVoiceState {
    // PERF: *maybe* caching this could be helpful? since perm checks will probably end up having to be run for every MediaData event?
    pub fn permissions(&self) -> Permissions {
        Permissions {
            video: self.permissions.video(),
            audio: self.permissions.speak() && !self.inner.muted(),
            deaf: self.inner.deafened(),
        }
    }
}
