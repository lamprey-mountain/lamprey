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
    formats::{FormatReader, SeekMode, SeekTo},
    io::MediaSourceStream,
    probe::Hint,
    units::TimeBase,
};

use super::{Frame, Producer, Source};
use crate::voice::VoiceError;

/// a media source backed by symphonia (e.g. an audio file or buffer)
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
        mss: MediaSourceStream,
        hint: Hint,
        seekable: bool,
    ) -> Result<Self, VoiceError> {
        Self::probe(mss, hint, seekable)
    }

    fn probe(mss: MediaSourceStream, hint: Hint, seekable: bool) -> Result<Self, VoiceError> {
        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &Default::default(),
            &Default::default(),
        )?;

        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec == codecs::CODEC_TYPE_OPUS)
            .ok_or(VoiceError::NoMatchingTrack)?;

        let track_id = track.id;
        let time_base = track
            .codec_params
            .time_base
            .unwrap_or_else(|| TimeBase::new(1, 48_000));
        let duration = track.codec_params.n_frames.map(|frames| {
            let t = time_base.calc_time(frames);
            Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac)
        });

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
                Ok(packet) => packet,
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

            if packet.track_id() == self.track_id {
                let ts = packet.ts();
                let t = self.time_base.calc_time(ts);
                self.position = Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac);

                let freq =
                    Frequency::new(self.time_base.denom).unwrap_or(Frequency::FORTY_EIGHT_KHZ);
                let time = MediaTime::new(ts, freq);

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

        let time = symphonia::core::units::Time {
            seconds: position.as_secs(),
            frac: f64::from(position.subsec_nanos()) / 1_000_000_000.0,
        };

        let seeked = self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time,
                track_id: Some(self.track_id),
            },
        )?;

        let t = self.time_base.calc_time(seeked.actual_ts);
        let actual = Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac);
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
