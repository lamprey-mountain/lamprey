use std::sync::{Arc, atomic::AtomicBool};

use futures::{Stream, StreamExt};

use crate::voice::{
    VoiceError,
    player::{
        node::{Handle, Node},
        rtc::Packet,
        util::MediaSource,
    },
};

pub struct Pause<N: Node> {
    source: N,
    paused: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct PauseHandle {
    paused: Arc<AtomicBool>,
}

impl Handle for PauseHandle {}

impl<N: Node> Pause<N> {
    pub fn new(source: N) -> Self {
        let paused = Arc::new(AtomicBool::new(false));
        Pause { source, paused }
    }
}

impl PauseHandle {
    pub fn paused(&self) -> bool {
        self.paused.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_paused(&self, v: bool) {
        self.paused.store(v, std::sync::atomic::Ordering::Relaxed);
    }
}

impl<N: Node> Node for Pause<N> {
    type Handle = PauseHandle;
    type Media = N::Media;

    fn handle(&self) -> Self::Handle {
        PauseHandle {
            paused: self.paused.clone(),
        }
    }
}

impl<N: Node> MediaSource<N::Media> for Pause<N> {
    fn stream<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<Packet<N::Media>, VoiceError>> + Send + 'a {
        let paused = self.paused.clone();

        self.source.stream().map(move |res| {
            res.map(|p| {
                if paused.load(std::sync::atomic::Ordering::Relaxed) {
                    Packet::empty(p.time)
                } else {
                    p
                }
            })
        })
    }
}
