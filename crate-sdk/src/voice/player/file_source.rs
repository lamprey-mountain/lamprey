use std::{
    io::Cursor,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use bytes::Bytes;
use common::v1::types::voice::MediaKind;
use futures::Stream;
use str0m::{
    format::Codec,
    media::{Frequency, MediaTime},
};
use symphonia::core::{
    codecs,
    formats::{FormatReader, SeekMode, SeekTo, probe::Hint},
    io::MediaSourceStream,
    units::{Time, TimeBase},
};

use super::{Frame, Producer, Source};
use crate::voice::VoiceError;

/// a media source backed by symphonia (e.g. an audio file or buffer)
///
/// currently, only opus tracks are supported. other codecs are ignored when probing.
pub struct FileSource {
    format: Box<dyn FormatReader>,
    track_id: u32,
    codec: Codec,
    time_base: TimeBase,
    duration: Option<Duration>,
    position: Duration,
    seekable: bool,
}

impl FileSource {
    /// open a media source from a file path
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, VoiceError> {
        let path = path.as_ref();
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        Self::probe(mss, hint, true)
    }

    /// open a media source from an in-memory buffer
    pub fn from_bytes(bytes: impl Into<Box<[u8]>>) -> Result<Self, VoiceError> {
        let cursor = Cursor::new(bytes.into());
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
        Self::probe(mss, Hint::new(), true)
    }

    /// open a media source from a symphonia `MediaSourceStream`
    pub fn from_stream(
        mss: MediaSourceStream<'static>,
        hint: Hint,
        seekable: bool,
    ) -> Result<Self, VoiceError> {
        Self::probe(mss, hint, seekable)
    }

    fn probe(
        mss: MediaSourceStream<'static>,
        hint: Hint,
        seekable: bool,
    ) -> Result<Self, VoiceError> {
        let format = symphonia::default::get_probe().probe(
            &hint,
            mss,
            Default::default(),
            Default::default(),
        )?;

        let track = format
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params
                    .as_ref()
                    .and_then(|params| params.audio())
                    .is_some_and(|audio| audio.codec == codecs::audio::well_known::CODEC_ID_OPUS)
            })
            .ok_or(VoiceError::NoMatchingTrack)?;

        let track_id = track.id;
        let time_base = track
            .time_base
            .unwrap_or_else(|| TimeBase::try_new(1, 48_000).unwrap());
        let duration = track
            .duration
            .and_then(|duration| time_base.calc_duration(duration))
            .map(std_time);

        Ok(Self {
            format,
            track_id,
            codec: Codec::Opus,
            time_base,
            duration,
            position: Duration::ZERO,
            seekable,
        })
    }
}

impl Stream for FileSource {
    type Item = Result<Frame, VoiceError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => return Poll::Ready(None),
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Poll::Ready(None);
                }
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    return Poll::Ready(None);
                }
                Err(e) => return Poll::Ready(Some(Err(VoiceError::Symphonia(e)))),
            };

            if packet.track_id == self.track_id {
                let ts = packet.pts;
                self.position = std_time(self.time_base.calc_time_saturating(ts));

                let freq = Frequency::new(self.time_base.denom.get())
                    .unwrap_or(Frequency::FORTY_EIGHT_KHZ);
                let time = MediaTime::new(ts.get() as u64, freq);

                return Poll::Ready(Some(Ok(Frame {
                    codec: self.codec,
                    time,
                    rid: None,
                    data: Bytes::copy_from_slice(&packet.data),
                })));
            }
        }
    }
}

impl Producer for FileSource {
    fn kind(&self) -> MediaKind {
        // TODO: support video streaming too
        MediaKind::Audio
    }
}

impl Source for FileSource {
    fn is_seekable(&self) -> bool {
        self.seekable
    }

    fn seek(&mut self, position: Duration) -> Result<Duration, VoiceError> {
        if !self.seekable {
            return Err(VoiceError::NotSeekable);
        }

        let time =
            Time::try_new(position.as_secs() as i64, position.subsec_nanos()).unwrap_or_default();

        let seeked = self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time,
                track_id: Some(self.track_id),
            },
        )?;

        let actual = std_time(self.time_base.calc_time_saturating(seeked.actual_ts));
        self.position = actual;
        Ok(actual)
    }

    fn duration(&self) -> Option<Duration> {
        self.duration
    }

    fn position(&self) -> Option<Duration> {
        Some(self.position)
    }
}

fn std_time(time: Time) -> Duration {
    let (secs, nanos) = time.parts();
    Duration::from_secs(secs.max(0) as u64) + Duration::from_nanos(u64::from(nanos))
}
