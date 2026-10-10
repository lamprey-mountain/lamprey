use common::{
    v1::types::voice::{MediaKind, Mid, TrackId, TrackKey},
    v2::types::UserId,
};
use futures::Stream;

use crate::voice::VoiceError;

/// a track that can be subscribed to
pub struct Inbound {
    // id: TrackId,
    // user_id: UserId,
    // kind: MediaKind,
    // ...
}

/// a track that you're publishing
pub struct Outbound {
    // id: TrackId,
    // kind: MediaKind,
    // ...
}

/// a stream of media from a track
///
/// when the last `InboundStream` referencing a track is dropped, that stream is
/// automatically unsubscribed
pub struct InboundStream {
    // ...
}

// NOTE: could i deduplicate Inbound/Outbound structs?
// maybe i could have Track and TrackWriter?
impl Inbound {
    /// get the kind of media this track contains
    pub fn kind(&self) -> MediaKind {
        todo!()
    }

    /// get which stream this track is associated with
    pub fn key(&self) -> TrackKey {
        todo!()
    }

    /// get the assigned track id
    pub fn track_id(&self) -> TrackId {
        todo!()
    }

    /// get which user is publishing this track
    pub fn user_id(&self) -> UserId {
        todo!()
    }

    /// stream media from this track
    pub async fn stream(&self) -> Result<InboundStream, VoiceError> {
        todo!()
    }
}

impl Outbound {
    /// get the kind of media this track contains
    pub fn kind(&self) -> MediaKind {
        todo!()
    }

    /// get which stream this track is associated with
    pub fn key(&self) -> TrackKey {
        todo!()
    }

    /// get the local mid of this track
    pub fn mid(&self) -> Mid {
        todo!()
    }

    /// get the assigned track id
    ///
    /// only exists after signalling
    pub fn track_id(&self) -> Option<TrackId> {
        todo!()
    }

    // /// create a new audio track
    // pub fn new_audio() -> Self;

    // /// create a new video track
    // pub fn new_video() -> Self;

    // /// Push encoded packets (e.g., Opus for audio, VP8 for video) directly to the SFU
    // pub async fn write_encoded(&self, packet: EncodedPacket) -> Result<(), VoiceError>;

    // /// (Optional) Push raw PCM if you build in an Opus encoder
    // pub async fn write_pcm(&self, pcm_data: &[i16]) -> Result<(), VoiceError>;

    // impl Sink for Outbound
    // how is simulcasting done?
}

impl InboundStream {
    /// get the assigned local mid
    pub fn mid(&self) -> Mid {
        todo!()
    }

    // // for video, request a pli keyframe?
    // pub fn request_keyframe(&self);
    //
    // select rid/simulcast layer
}

impl Stream for InboundStream {
    type Item = (); // TODO: rtc packet type

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}
