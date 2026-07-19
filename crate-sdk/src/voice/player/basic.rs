/// a basic audio player
pub struct BasicPlayer {
    // TODO
}

impl BasicPlayer {
    pub fn new() -> Self {
        todo!()
    }

    pub fn paused(&self) -> bool {
        todo!()
    }

    pub fn set_paused(&self, paused: bool) {
        todo!()
    }

    pub fn volume(&self) -> f32 {
        todo!()
    }

    pub fn set_volume(&self, volume: f32) {
        todo!()
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
