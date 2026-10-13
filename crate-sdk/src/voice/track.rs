use common::{
    v1::types::voice::{MediaKind, Mid, TrackId, TrackKey},
    v2::types::UserId,
};
use futures::Stream;

use crate::voice::VoiceError;

#[derive(Debug, Clone)]
pub enum Track {
    Inbound(Inbound),
    Outbound(Outbound),
}

/// a track that can be subscribed to
#[derive(Debug, Clone)]
pub struct Inbound {
    pub(super) id: TrackId,
    pub(super) user_id: UserId,
    pub(super) kind: MediaKind,
    pub(super) key: TrackKey,
    // etc...
}

/// a track that you're publishing
#[derive(Debug, Clone)]
pub struct Outbound {
    pub(super) mid: Mid,
    pub(super) id: Option<TrackId>,
    pub(super) kind: MediaKind,
    pub(super) key: TrackKey,
    // etc...
}

/// a stream of media from a track
///
/// when the last `InboundStream` referencing a track is dropped, that stream is
/// automatically unsubscribed
pub struct InboundStream {
    // ...
}

impl Track {
    /// get the kind of media this track contains
    #[inline]
    pub fn kind(&self) -> MediaKind {
        match self {
            Track::Inbound(t) => t.kind(),
            Track::Outbound(t) => t.kind(),
        }
    }

    /// get which stream this track is associated with
    #[inline]
    pub fn key(&self) -> &TrackKey {
        match self {
            Track::Inbound(t) => t.key(),
            Track::Outbound(t) => t.key(),
        }
    }

    /// get the assigned track id
    #[inline]
    pub fn track_id(&self) -> Option<TrackId> {
        match self {
            Track::Inbound(t) => Some(t.track_id()),
            Track::Outbound(t) => t.track_id(),
        }
    }
}

// NOTE: could i deduplicate Inbound/Outbound structs?
// maybe i could have Track and TrackWriter?
impl Inbound {
    /// get the kind of media this track contains
    #[inline]
    pub fn kind(&self) -> MediaKind {
        self.kind
    }

    /// get which stream this track is associated with
    #[inline]
    pub fn key(&self) -> &TrackKey {
        &self.key
    }

    /// get the assigned track id
    #[inline]
    pub fn track_id(&self) -> TrackId {
        self.id
    }

    /// get which user is publishing this track
    #[inline]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// stream media from this track
    pub async fn stream(&self) -> Result<InboundStream, VoiceError> {
        todo!()
    }
}

impl Outbound {
    /// get the kind of media this track contains
    #[inline]
    pub fn kind(&self) -> MediaKind {
        self.kind
    }

    /// get which stream this track is associated with
    #[inline]
    pub fn key(&self) -> &TrackKey {
        &self.key
    }

    /// get the local mid of this track
    #[inline]
    pub fn mid(&self) -> Mid {
        self.mid
    }

    /// get the assigned track id
    ///
    /// only exists after signalling
    #[inline]
    pub fn track_id(&self) -> Option<TrackId> {
        self.id
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

    /// request a pli keyframe from the sender
    ///
    /// only valid for video
    pub fn request_keyframe(&self) {
        todo!()
    }

    /// select which simulcast layer to receive
    ///
    /// only valid for video
    pub fn request_layer(&self, _rid: str0m::media::Rid) {
        // TODO: how does this work? do i use str0m's rid type or a custom lamprey rid type? probably just str0m's Rid.
        todo!()
    }
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
