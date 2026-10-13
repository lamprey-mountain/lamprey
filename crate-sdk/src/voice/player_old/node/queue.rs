use std::time::Duration;

use crate::voice::player::{node::Node, util::MediaSource};

// TODO: implement
// TODO: handle mixed queues

pub struct Queue {
    // sources: Vec<Box<dyn Node>>,
}

#[derive(Clone)]
pub struct QueueHandle {
    // TODO
}

enum QueueEntryStatus {
    Playing,
    Buffering,
}

// pub struct QueueEntryHandle;
// pub struct QueueEntryId;

impl QueueHandle {
    // /// start playing the next track
    // pub fn push(&self, source: MediaSource<_>) {
    //     todo!()
    // }

    // TODO: add
    // insert(usize, source)
    // get(usize)
    // remove(usize)
    // len() -> usize
    // clear_played() -- remove all played tracks
    // clear() -- remove all tracks, stop current track

    // position() -> usize -- get the current position in the queue

    /// start playing the next track
    pub fn seek_next(&self) {
        self.seek_by(1)
    }

    /// start playing the previous track
    pub fn seek_prev(&self) {
        self.seek_by(-1)
    }

    pub fn seek_by(&self, count: isize) {
        todo!()
    }

    pub fn seek_to(&self, position: usize) {
        todo!()
    }

    /// get the total duration of all tracks in this queue
    pub fn total_duration(&self) -> Duration {
        todo!()
    }

    // pub fn current(&self) -> Option<usize> {
    // pub fn current(&self) -> Option<&QueueEntry> {
    //     todo!()
    // }
}

// impl QueueEntry {
//   // TODO: add?
//   // remove()
//   // metadata()
// }
