use futures::{Stream, StreamExt};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use crate::voice::{
    VoiceError,
    player::{
        node::{Handle, Node},
        rtc::Packet,
        util::MediaSource,
    },
};

pub struct Volume<N: Node> {
    source: N,
    volume: Arc<AtomicU32>,
}

#[derive(Clone)]
pub struct VolumeHandle {
    volume: Arc<AtomicU32>,
}

impl Handle for VolumeHandle {}

impl VolumeHandle {
    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }

    pub fn set_volume(&self, volume: f32) {
        self.volume.store(volume.to_bits(), Ordering::Relaxed);
    }
}

impl<N: Node> Volume<N> {
    pub fn new(source: N) -> Self {
        let volume = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        Volume { source, volume }
    }
}

impl<N: Node> Node for Volume<N> {
    type Handle = VolumeHandle;
    type Media = N::Media;

    fn handle(&self) -> Self::Handle {
        VolumeHandle {
            volume: self.volume.clone(),
        }
    }
}

impl<N: Node> MediaSource<N::Media> for Volume<N> {
    fn stream<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<Packet<N::Media>, VoiceError>> + Send + 'a {
        let volume = self.volume.clone();

        self.source.stream().map(move |res| {
            res.map(|p| {
                let vol = f32::from_bits(volume.load(Ordering::Relaxed));
                if (vol - 1.0).abs() < f32::EPSILON {
                    p
                } else {
                    // TODO: apply volume to PCM data
                    p
                }
            })
        })
    }
}
