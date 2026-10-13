use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};

use common::v1::types::voice::MediaKind;
use futures::Stream;
use parking_lot::Mutex;
use tokio::{
    sync::watch,
    time::{Interval, MissedTickBehavior},
};
use tokio_stream::wrappers::WatchStream;

use super::{Frame, Producer, Source};
use crate::voice::VoiceError;

struct PlayerShared {
    paused: AtomicBool,
    volume: AtomicU32,
    position_nanos: AtomicU64,
    duration_nanos: AtomicU64,
    seekable: AtomicBool,
    queue: Mutex<PlayerQueue>,
    wake_tx: watch::Sender<()>,
}

#[derive(Default)]
struct PlayerQueue {
    sources: VecDeque<Box<dyn Source>>,
    seek_request: Option<Duration>,
    skip_request: bool,
    stop_request: bool,
}

/// A `Player` is a cheaply cloneable handle used to control a
/// [`PlayerProducer`]. It can control playback (play, pause, seek, etc...) and
/// retrieve state like playback position or media duration.
///
/// `Player`s have a builtin queue. You can enqueue multiple different media
/// sources, and they will play in sequence.
///
/// This doesn't produce any media by itself. You have to publish the associated
/// [`PlayerProducer`] to a voice channel for it to do anything.
#[derive(Clone)]
pub struct Player {
    shared: Arc<PlayerShared>,
}

/// The [`Producer`] side of a [`Player`] that can be passed to
/// [`Voice::publish`](crate::voice::Voice::publish).
pub struct PlayerProducer {
    kind: MediaKind,
    shared: Arc<PlayerShared>,
    wake_rx: WatchStream<()>,
    current: Option<Box<dyn Source>>,
    interval: Interval,
}

impl Player {
    /// create a new audio player and its associated [`PlayerProducer`]
    pub fn new_audio() -> (Self, PlayerProducer) {
        Self::new(MediaKind::Audio, Duration::from_millis(20))
    }

    /// create a new player for the given media kind and frame interval
    ///
    /// `frame_interval` is used to pace frames. for example, opus should
    /// typically be paced to 20ms per frame.
    pub fn new(kind: MediaKind, frame_interval: Duration) -> (Self, PlayerProducer) {
        let (wake_tx, wake_rx) = watch::channel(());
        let shared = Arc::new(PlayerShared {
            paused: AtomicBool::new(false),
            volume: AtomicU32::new(1.0f32.to_bits()),
            position_nanos: AtomicU64::new(0),
            duration_nanos: AtomicU64::new(u64::MAX),
            seekable: AtomicBool::new(false),
            queue: Mutex::new(PlayerQueue::default()),
            wake_tx,
        });

        let mut interval = tokio::time::interval(frame_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let player = Self {
            shared: shared.clone(),
        };
        let producer = PlayerProducer {
            kind,
            shared,
            wake_rx: WatchStream::new(wake_rx),
            current: None,
            interval,
        };

        (player, producer)
    }

    /// Enqueues a media source to be played.
    #[doc(alias = "push")]
    pub fn enqueue(&self, source: impl Source) {
        self.shared.queue.lock().sources.push_back(Box::new(source));
        let _ = self.shared.wake_tx.send(());
    }

    /// Plays a new source, replacing the current track and clearing the queue.
    pub fn play(&self, source: impl Source) {
        let mut q = self.shared.queue.lock();
        q.sources.clear();
        q.sources.push_back(Box::new(source));
        q.skip_request = true;
        drop(q);
        let _ = self.shared.wake_tx.send(());
    }

    /// Skips the currently playing source and advances to the next source in the queue.
    #[doc(alias = "next")]
    pub fn skip(&self) {
        self.shared.queue.lock().skip_request = true;
        let _ = self.shared.wake_tx.send(());
    }

    /// Clears the queue and stops the player.
    pub fn stop(&self) {
        let mut q = self.shared.queue.lock();
        q.sources.clear();
        q.stop_request = true;
        drop(q);
        let _ = self.shared.wake_tx.send(());
    }

    /// Returns whether the player is currently paused.
    pub fn paused(&self) -> bool {
        self.shared.paused.load(Ordering::Relaxed)
    }

    /// Pauses or unpauses the player.
    #[doc(alias = "resume")]
    pub fn set_paused(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Relaxed);
        if !paused {
            let _ = self.shared.wake_tx.send(());
        }
    }

    /// get the current volume multiplier
    pub fn volume(&self) -> f32 {
        f32::from_bits(self.shared.volume.load(Ordering::Relaxed))
    }

    /// set the volume multiplier
    ///
    /// this is **not clamped** and can go above `1.0`. you can mute audio with
    /// a volume of `0.0`.
    pub fn set_volume(&self, volume: f32) {
        self.shared
            .volume
            .store(volume.to_bits(), Ordering::Relaxed);
    }

    /// request a seek on the currently playing source
    pub fn seek(&self, position: Duration) {
        self.shared.queue.lock().seek_request = Some(position);
        let _ = self.shared.wake_tx.send(());
    }

    /// whether the currently playing source is seekable
    pub fn is_seekable(&self) -> bool {
        self.shared.seekable.load(Ordering::Relaxed)
    }

    /// get the current playback position of the active source
    pub fn position(&self) -> Duration {
        Duration::from_nanos(self.shared.position_nanos.load(Ordering::Relaxed))
    }

    /// get the duration of the active source, if known
    pub fn duration(&self) -> Option<Duration> {
        let nanos = self.shared.duration_nanos.load(Ordering::Relaxed);
        if nanos == u64::MAX {
            None
        } else {
            Some(Duration::from_nanos(nanos))
        }
    }

    /// get the number of sources waiting in the queue
    pub fn queue_len(&self) -> usize {
        self.shared.queue.lock().sources.len()
    }

    // TODO: fn queue_duration: the total duration until the player's queue runs out of media to play
    // TODO: fn prev: reverse of skip
    // NOTE: depending on how its implemented, volume could need to be renamed to gain
}

impl PlayerProducer {
    /// update shared atomics with metadata about the current source
    fn update_metadata(&self) {
        if let Some(src) = &self.current {
            self.shared
                .seekable
                .store(src.is_seekable(), Ordering::Relaxed);
            let pos = src.position().unwrap_or(Duration::ZERO).as_nanos() as u64;
            self.shared.position_nanos.store(pos, Ordering::Relaxed);
            let dur = src
                .duration()
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(u64::MAX);
            self.shared.duration_nanos.store(dur, Ordering::Relaxed);
        } else {
            self.shared.seekable.store(false, Ordering::Relaxed);
            self.shared.position_nanos.store(0, Ordering::Relaxed);
            self.shared
                .duration_nanos
                .store(u64::MAX, Ordering::Relaxed);
        }
    }
}

// TODO: surely there's a cleaner way of implementing this?
impl Stream for PlayerProducer {
    type Item = Result<Frame, VoiceError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            while let Poll::Ready(Some(())) = Pin::new(&mut self.wake_rx).poll_next(cx) {}

            {
                let mut q = self.shared.queue.lock();
                if q.stop_request {
                    q.stop_request = false;
                    drop(q);
                    self.current = None;
                    self.update_metadata();
                    return Poll::Ready(None);
                }

                if q.skip_request {
                    q.skip_request = false;
                    let next = q.sources.pop_front();
                    drop(q);
                    self.current = next;
                    self.update_metadata();
                } else if self.current.is_none() {
                    let next = q.sources.pop_front();
                    drop(q);
                    self.current = next;
                    self.update_metadata();
                } else {
                    drop(q);
                }

                let seek_pos = self.shared.queue.lock().seek_request.take();
                if let Some(seek_pos) = seek_pos {
                    if let Some(src) = &mut self.current {
                        if let Err(e) = src.seek(seek_pos) {
                            return Poll::Ready(Some(Err(e)));
                        }
                        self.update_metadata();
                    }
                }
            }

            if self.shared.paused.load(Ordering::Relaxed) || self.current.is_none() {
                match Pin::new(&mut self.wake_rx).poll_next(cx) {
                    Poll::Ready(Some(())) => continue,
                    Poll::Ready(None) => return Poll::Ready(None),
                    Poll::Pending => return Poll::Pending,
                }
            }

            if self.interval.poll_tick(cx).is_pending() {
                return Poll::Pending;
            }

            let frame_res = {
                let src = self.current.as_mut().expect("checked Some above");
                Pin::new(&mut **src).poll_next(cx)
            };

            match frame_res {
                Poll::Ready(Some(Ok(frame))) => {
                    self.update_metadata();
                    return Poll::Ready(Some(Ok(frame)));
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(e)));
                }
                Poll::Ready(None) => {
                    self.current = None;
                    self.update_metadata();
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl Producer for PlayerProducer {
    fn kind(&self) -> MediaKind {
        self.kind
    }
}
