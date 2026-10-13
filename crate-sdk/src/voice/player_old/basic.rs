use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// a basic audio player
pub struct BasicPlayer {
    paused: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
}

impl BasicPlayer {
    pub fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU32::new(1.0f32.to_bits())),
        }
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }

    pub fn set_volume(&self, volume: f32) {
        self.volume.store(volume.to_bits(), Ordering::Relaxed);
    }

    // TODO: seeking
    // fn position() -> f32
    // fn duration() -> f32
    // fn is_seekable() -> bool
    // fn seek_to(f32)
    // fn seek_by(f32)

    // TODO: queueing
    // fn play(impl Into CreateSource)
    // fn queue() -> Queue
    // fn queue_next()
    // fn queue_prev()
    // fn queue_len()
    // fn queue_duration()

    // TODO: looping
    // fn loop() -> Loop
    // fn set_loop(Loop)

    // TODO: get track metadata
    // fn track() -> track metadata
}
