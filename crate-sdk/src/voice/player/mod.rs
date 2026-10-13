// TODO: rename this to mod publish?

use std::time::Duration;

use bytes::Bytes;
use common::v1::types::voice::{KeyframeRequestKind, MediaKind, TrackKey};
use futures::{FutureExt, Stream, future::BoxFuture};
use str0m::{
    format::Codec,
    media::{MediaTime, Rid},
};
use tokio::sync::oneshot;

use crate::voice::{Outbound, Voice, VoiceError, driver::RtcCommand};

mod file_source;
mod player;
mod switchboard;

pub use file_source::FileSource;
pub use player::{Player, PlayerProducer};
pub use switchboard::Switchboard;

/// a stream of media frames that can be sent to a voice channel
pub trait Producer: Stream<Item = Result<Frame, VoiceError>> + Send + Unpin + 'static {
    /// request a keyframe from the underlying source
    ///
    /// a `rid` of `None` defaults to requesting a keyframe on the producer's
    /// currently active layer, if it has one. the default implementation is
    /// a no-op.
    fn request_keyframe(&mut self, _kind: KeyframeRequestKind, _rid: Option<Rid>) {}

    /// select the simulcast layer identified by the given rid
    ///
    /// the default implementation is a no-op.
    fn select_layer(&mut self, _rid: Rid) {}

    /// the kind of media produced by this producer
    fn kind(&self) -> MediaKind;
}

/// a source that media can be streamed from
pub trait Source: Producer {
    /// return whether this source supports seeking
    fn is_seekable(&self) -> bool {
        false
    }

    /// seek to a timestamp in the source
    fn seek(&mut self, _position: Duration) -> Result<Duration, VoiceError> {
        Err(VoiceError::NotSeekable)
    }

    /// get the total duration of this source, if known
    fn duration(&self) -> Option<Duration> {
        None
    }

    /// get the current playback position of this source, if known
    fn position(&self) -> Option<Duration> {
        None
    }
}

/// a frame of media
#[derive(Debug, Clone)]
pub struct Frame {
    pub codec: Codec,
    pub time: MediaTime,
    pub rid: Option<Rid>,
    pub data: Bytes,
}

/// builder for publishing an outbound media track
pub struct PublishBuilder<'a> {
    voice: &'a Voice,
    producer: Box<dyn Producer>,
    key: TrackKey,
}

impl<'a> PublishBuilder<'a> {
    pub(crate) fn new<P: Producer>(voice: &'a Voice, producer: P) -> Self {
        Self {
            voice,
            producer: Box::new(producer),
            key: TrackKey::User,
        }
    }

    /// set which stream this track should be associated with (defaults to [`TrackKey::User`])
    pub fn key(mut self, key: TrackKey) -> Self {
        self.key = key;
        self
    }

    async fn send(self) -> Result<Outbound, VoiceError> {
        let (tx, rx) = oneshot::channel();
        self.voice
            .state
            .tx
            .send(RtcCommand::Publish {
                producer: self.producer,
                key: self.key,
                reply: tx,
            })
            .await
            .map_err(|_| VoiceError::ConnectionClosed)?;

        rx.await.map_err(|_| VoiceError::ConnectionClosed)?
    }
}

/// awaiting the builder publishes the track and resolves to the resulting
/// [`Outbound`]
impl<'a> IntoFuture for PublishBuilder<'a> {
    type Output = Result<Outbound, VoiceError>;
    type IntoFuture = BoxFuture<'a, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        self.send().boxed()
    }
}
