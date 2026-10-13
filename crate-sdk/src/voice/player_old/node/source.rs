use std::{io::Cursor, marker::PhantomData, time::Duration};

use futures::{Stream, StreamExt};
use str0m::media::{Frequency, MediaTime};
use symphonia::core::formats::{FormatReader, Track};
use tokio::time::{MissedTickBehavior, interval};

use crate::voice::{
    VoiceError,
    player::{
        node::{Handle, Node},
        rtc::Packet,
        util::{Audio, CreateSource, MediaKind, MediaSource, Video},
    },
};

// TODO: seal trait
// TODO: move to util
pub trait Seekable: Send + Sync + 'static {
    fn is_seekable() -> bool;
}

pub struct CanSeek;
pub struct NoSeek;

impl Seekable for CanSeek {
    fn is_seekable() -> bool {
        true
    }
}

impl Seekable for NoSeek {
    fn is_seekable() -> bool {
        false
    }
}

pub struct Source<M: MediaKind, S: Seekable = NoSeek> {
    format: Box<dyn FormatReader>,
    track: Track,
    // seek_req: Arc<Mutex<Option<Duration>>>, // set by handle, drained in stream()
    _media: PhantomData<M>,
    _seek: PhantomData<S>,
}

pub struct SourceHandle<M: MediaKind, S: Seekable> {
    // seek_req: Arc<Mutex<Option<Duration>>>,
    _media: PhantomData<M>,
    _seek: PhantomData<S>,
}

impl<M: MediaKind, S: Seekable> Clone for SourceHandle<M, S> {
    fn clone(&self) -> Self {
        Self {
            _media: PhantomData,
            _seek: PhantomData,
        }
    }
}

impl<M: MediaKind, S: Seekable> Handle for SourceHandle<M, S> {}

impl<M: MediaKind, S: Seekable> Source<M, S> {
    fn new(format: Box<dyn FormatReader>, track: Track) -> Self {
        Self {
            format,
            track,
            _media: PhantomData,
            _seek: PhantomData,
        }
    }
}

impl<M: MediaKind> Source<M, NoSeek> {
    pub fn from_reader(mss: symphonia::core::io::MediaSourceStream) -> Result<Self, VoiceError> {
        let hint = symphonia::core::probe::Hint::new();

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
            .find(|t| M::matches_codec(t.codec_params.codec))
            .ok_or(VoiceError::NoMatchingTrack)?
            .clone();

        Ok(Self::new(format, track))
    }
}

impl<M: MediaKind> Source<M, CanSeek> {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, VoiceError> {
        let source = CreateSource::Path(path.as_ref());
        Self::new_from_source(source)
    }

    pub fn from_buffer(bytes: impl Into<Box<[u8]>>) -> Result<Self, VoiceError> {
        let source = CreateSource::Memory(Cursor::new(bytes.into()));
        Self::new_from_source(source)
    }

    fn new_from_source(source: CreateSource) -> Result<Self, VoiceError> {
        let hint = source.hint();
        let mss = source.mss()?;
        // if mss.is_seekable() {}

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
            .find(|t| M::matches_codec(t.codec_params.codec))
            .ok_or(VoiceError::NoMatchingTrack)?
            .clone();

        Ok(Self::new(format, track))
    }
}

impl<M: MediaKind, S: Seekable> SourceHandle<M, S> {
    /// is this media source seekable?
    pub fn is_seekable(&self) -> bool {
        S::is_seekable()
    }

    /// try to convert this media source into a seekable source
    pub fn into_seekable(self) -> Result<SourceHandle<M, CanSeek>, Self> {
        todo!()
    }

    // fn stop()
}

impl<M: MediaKind> SourceHandle<M, CanSeek> {
    // fn seek_to(f32)
    // fn seek_by(f32)
}

impl<M: MediaKind, S: Seekable> MediaSource<M> for Source<M, S> {
    fn stream<'a>(&'a mut self) -> impl Stream<Item = Result<Packet<M>, VoiceError>> + Send + 'a {
        // TODO: don't panic/unwrap
        let track_id = self.track.id;
        let base = self.track.codec_params.time_base.unwrap();
        let mut interval = tokio::time::interval(Duration::from_millis(20));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        futures::stream::unfold(
            (&mut self.format, interval),
            move |(format, mut interval)| async move {
                loop {
                    let packet = match format.next_packet() {
                        Ok(packet) => packet,
                        Err(symphonia::core::errors::Error::IoError(e))
                            if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                        {
                            return None;
                        }
                        Err(e) => {
                            return Some((Err(VoiceError::from(e)), (format, interval)));
                        }
                    };

                    if packet.track_id() == track_id {
                        let time = MediaTime::new(packet.ts(), Frequency::new(base.denom).unwrap());
                        // PERF: don't clone
                        let data = packet.data.clone();
                        let p = Packet {
                            data,
                            time,
                            _kind: PhantomData,
                        };
                        interval.tick().await;
                        return Some((Ok(p), (format, interval)));
                    }

                    // packet didn't match, go around the loop again
                }
            },
        )
    }
}

impl<M: MediaKind, S: Seekable> Node for Source<M, S> {
    type Handle = SourceHandle<M, S>;
    type Media = M;

    fn handle(&self) -> Self::Handle {
        SourceHandle {
            _media: PhantomData,
            _seek: PhantomData,
        }
    }
}

pub type AudioSource<S> = Source<Audio, S>;
pub type AudioSourceHandle<S> = SourceHandle<Audio, S>;
pub type VideoSource<S> = Source<Video, S>;
pub type VideoSourceHandle<S> = SourceHandle<Video, S>;
pub type SeekableAudioSource = AudioSource<CanSeek>;
pub type SeekableAudioSourceHandle = AudioSourceHandle<CanSeek>;
pub type SeekableVideoSource = VideoSource<CanSeek>;
pub type SeekableVideoSourceHandle = VideoSourceHandle<CanSeek>;
