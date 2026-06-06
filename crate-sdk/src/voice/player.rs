use std::{
    io::Cursor,
    path::Path,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::{stream, Stream, StreamExt};
use str0m::media::{Frequency, MediaTime};
use symphonia::core::{
    formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};
use tokio::time::MissedTickBehavior;

use crate::voice::VoiceError;

// TODO: decode audio data (wav, mp3, opus, vorbis, flac) to pcm
// TODO: encode audio data to opus for rtc

#[derive(Debug)]
pub struct AudioPacket {
    pub data: Box<[u8]>,
    pub time: MediaTime,
}

#[derive(Debug)]
pub struct VideoPacket {
    pub data: Box<[u8]>,
    pub time: MediaTime,
}

/// a playable source of audio
pub trait AudioSource: Send + Sync {
    fn stream_audio<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<AudioPacket, VoiceError>> + Send + 'a;
}

/// a playable source of video
pub trait VideoSource: Send + Sync {
    fn stream_video<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<VideoPacket, VoiceError>> + Send + 'a;
}

/// play an encoded audio file
pub struct AudioFile {
    track: symphonia::core::formats::Track,
    format: Box<dyn symphonia::core::formats::FormatReader>,
}

/// transform another audio stream
pub struct AudioTransform<S> {
    source: S,
    shared: Arc<AudioTransformShared>,
}

#[derive(Debug)]
pub struct AudioTransformShared {
    paused: AtomicBool,
    volume: AtomicU32,
}

/// a handle to transform a live `AudioTransform`
#[derive(Debug, Clone)]
pub struct AudioTransformHandle {
    shared: Arc<AudioTransformShared>,
}

/// transform another video stream
pub struct VideoTransform<S> {
    source: S,
    shared: Arc<VideoTransformShared>,
}

#[derive(Debug)]
pub struct VideoTransformShared {
    paused: AtomicBool,
}

/// a handle to transform a live `VideoTransform`
#[derive(Debug, Clone)]
pub struct VideoTransformHandle {
    shared: Arc<VideoTransformShared>,
}

impl<S: VideoSource> VideoTransform<S> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            shared: Arc::new(VideoTransformShared {
                paused: AtomicBool::new(false),
            }),
        }
    }

    pub fn handle(&self) -> VideoTransformHandle {
        VideoTransformHandle {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl VideoTransformHandle {
    #[inline]
    pub fn paused(&self) -> bool {
        self.shared.paused.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_paused(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Relaxed);
    }
}

impl<S: VideoSource> VideoSource for VideoTransform<S> {
    fn stream_video<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<VideoPacket, VoiceError>> + Send + 'a {
        let handle = self.handle();

        self.source.stream_video().map(move |res| {
            let handle = handle.clone();
            match res {
                Ok(p) => {
                    if handle.paused() {
                        Ok(VideoPacket {
                            data: vec![].into_boxed_slice(),
                            time: p.time,
                        })
                    } else {
                        Ok(p)
                    }
                }
                Err(e) => Err(e),
            }
        })
    }
}

impl AudioFile {
    /// create a new audio source that plays a file
    pub fn new_from_path(path: impl AsRef<Path>) -> Result<Self, VoiceError> {
        let mut hint = Hint::new();
        if let Some(ext) = path.as_ref().extension().and_then(|ext| ext.to_str()) {
            hint.with_extension(ext);
        }

        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        Self::new(hint, mss)
    }

    /// create a new audio source that plays an in memory file
    pub fn new_from_bytes(bytes: impl Into<Box<[u8]>>) -> Result<Self, VoiceError> {
        let hint = Hint::new();
        let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes.into())), Default::default());
        Self::new(hint, mss)
    }

    fn new(hint: Hint, mss: MediaSourceStream) -> Result<Self, VoiceError> {
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();
        // TODO: don't panic
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .expect("unsupported format");
        let format = probed.format;

        // TODO: allow other formats
        // TODO: handle multiple formats in one file
        let track = format
            .tracks()
            .iter()
            // what formats are supported...?
            // .find(|t| matches!(t.codec_params.codec, symphonia::core::codecs::CODEC_TYPE_OPUS | symphonia::core::codecs::CODEC_TYPE_VORBIS))
            .find(|t| t.codec_params.codec == symphonia::core::codecs::CODEC_TYPE_OPUS)
            .expect("no supported audio tracks")
            .clone();

        // let decoder = symphonia::default::get_codecs()
        //     .make(&track.codec_params, &DecoderOptions::default())
        //     .unwrap();

        Ok(Self { track, format })
    }
}

impl<S: AudioSource> AudioTransform<S> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            shared: Arc::new(AudioTransformShared {
                paused: AtomicBool::new(false),
                volume: AtomicU32::new(1.0f32.to_bits()),
            }),
        }
    }

    pub fn handle(&self) -> AudioTransformHandle {
        AudioTransformHandle {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl AudioTransformHandle {
    #[inline]
    pub fn paused(&self) -> bool {
        self.shared.paused.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn volume(&self) -> f32 {
        f32::from_bits(self.shared.volume.load(Ordering::Relaxed))
    }

    #[inline]
    pub fn set_paused(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_volume(&self, volume: f32) {
        self.shared
            .volume
            .store(volume.to_bits(), Ordering::Relaxed);
    }
}

impl AudioSource for AudioFile {
    fn stream_audio<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<AudioPacket, VoiceError>> + Send + 'a {
        // TODO: don't panic
        let track_id = self.track.id;
        let base = self.track.codec_params.time_base.unwrap();
        let mut interval = tokio::time::interval(Duration::from_millis(20));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        stream::unfold((self, interval), move |(state, mut interval)| async move {
            loop {
                let packet = match state.format.next_packet() {
                    Ok(packet) => packet,
                    Err(symphonia::core::errors::Error::IoError(e))
                        if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                    {
                        return None
                    }
                    Err(e) => {
                        return Some((Err(VoiceError::from(e)), (state, interval)));
                    }
                };

                if packet.track_id() == track_id {
                    let time = MediaTime::new(packet.ts(), Frequency::new(base.denom).unwrap());
                    // PERF: don't clone
                    let data = packet.data.clone();
                    let p = AudioPacket { data, time };
                    interval.tick().await;
                    return Some((Ok(p), (state, interval)));
                }

                // packet didn't match, go around the loop again
            }
        })
    }
}

impl<S: AudioSource> AudioSource for AudioTransform<S> {
    fn stream_audio<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<AudioPacket, VoiceError>> + Send + 'a {
        let handle = self.handle();

        self.source.stream_audio().map(move |res| {
            let handle = handle.clone();
            match res {
                Ok(p) => {
                    // TODO: handle handle.volume()
                    if handle.paused() {
                        Ok(AudioPacket {
                            data: vec![].into_boxed_slice(),
                            time: p.time,
                        })
                    } else {
                        Ok(p)
                    }
                }
                Err(e) => Err(e),
            }
        })
    }
}

/// play an encoded video file
pub struct VideoFile {
    track: symphonia::core::formats::Track,
    format: Box<dyn symphonia::core::formats::FormatReader>,
}

impl VideoFile {
    /// create a new video source that plays a file
    pub fn new_from_path(path: impl AsRef<Path>) -> Result<(Self, Option<AudioFile>), VoiceError> {
        let mut hint = Hint::new();
        if let Some(ext) = path.as_ref().extension().and_then(|ext| ext.to_str()) {
            hint.with_extension(ext);
        }

        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        Self::new(hint, mss)
    }

    pub fn new_from_bytes(
        bytes: impl Into<Box<[u8]>>,
    ) -> Result<(Self, Option<AudioFile>), VoiceError> {
        let hint = Hint::new();
        let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes.into())), Default::default());
        Self::new(hint, mss)
    }

    fn new(hint: Hint, mss: MediaSourceStream) -> Result<(Self, Option<AudioFile>), VoiceError> {
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .map_err(|e| VoiceError::Other)?; // TODO: fix error mapping
        let format = probed.format;

        let video_track = format
            .tracks()
            .iter()
            // FIXME: symphonia doesnt support video
            // .find(|t| matches!(t.codec_params.codec, ???))
            .find(|t| todo!())
            .ok_or(VoiceError::Other)? // TODO: fix error mapping
            .clone();

        let video = VideoFile {
            track: video_track,
            format: format,
        };

        // TODO: handle audio if present

        Ok((video, None))
    }
}

impl VideoSource for VideoFile {
    fn stream_video<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<VideoPacket, VoiceError>> + Send + 'a {
        futures_util::stream::empty()
    }
}

// /// a playable source of video + audio
// pub trait MediaSource: AudioSource + VideoSource {}

/// a video file with optional media
pub struct MediaFile {
    audio: Option<AudioFile>,
    video: VideoFile,
}

impl AudioSource for MediaFile {
    fn stream_audio<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<AudioPacket, VoiceError>> + Send + 'a {
        let stream: Pin<Box<dyn Stream<Item = _> + Send + 'a>> =
            if let Some(audio) = &mut self.audio {
                Box::pin(audio.stream_audio())
            } else {
                Box::pin(futures_util::stream::empty())
            };
        stream
    }
}

impl VideoSource for MediaFile {
    fn stream_video<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<VideoPacket, VoiceError>> + Send + 'a {
        self.video.stream_video()
    }
}
