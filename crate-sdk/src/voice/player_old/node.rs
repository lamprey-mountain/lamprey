use std::{
    marker::PhantomData,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use futures::StreamExt;
use futures_util::{Stream, stream};
use str0m::media::{Frequency, MediaTime};
use tokio::time::MissedTickBehavior;

use crate::voice::{
    VoiceError,
    player::{Audio, MediaKind, Packet, Source, player::PlayerInner, util::NodeKey},
};

// TODO: rename to NodeData?
pub struct Node<M: MediaKind, K: NodeKind> {
    kind: K,
    player: Arc<PlayerInner>,
    _phantom: PhantomData<M>,
}

/// a handle to a node
pub struct NodeHandle<M: MediaKind, K: NodeKind> {
    key: NodeKey,
    player: Arc<PlayerInner>,
    _kind: PhantomData<(M, K)>,
}

// TODO: rename to Node
pub trait NodeKind {
    // TODO: ???
    // type Handle;
}

pub struct Pause {
    source: Node<Audio, AudioSource>, // TODO: generics
    paused: Arc<AtomicBool>,
}

pub struct PauseHandle {
    paused: Arc<AtomicBool>,
}

pub struct Volume;

pub struct AudioSource {
    track: symphonia::core::formats::Track,
    format: Box<dyn symphonia::core::formats::FormatReader>,
}

pub struct VideoSource {
    // TODO
}

impl NodeKind for Pause {
    // type Handle = PauseHandle;
}

impl NodeKind for Volume {}
impl NodeKind for AudioSource {}
impl NodeKind for VideoSource {}

// impl Source for destination Node?

impl<M: MediaKind> Node<M, Pause> {
    // TODO
}

impl<M: MediaKind> NodeHandle<'_, M, Pause> {
    pub fn paused(&self) -> bool {
        todo!()
    }

    pub fn set_paused(&self, paused: bool) {
        todo!()
    }
}

// impl<M: MediaKind> Source<M> for Node<M, Pause> {
impl Source<Audio> for Node<Audio, Pause> {
    fn stream<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<Packet<Audio>, VoiceError>> + Send + 'a {
        let paused = Arc::clone(&self.kind.paused);

        self.kind.source.stream().map(move |res| {
            res.map(|p| {
                if paused.load(std::sync::atomic::Ordering::Relaxed) {
                    Packet {
                        data: vec![].into_boxed_slice(),
                        time: p.time,
                        _kind: PhantomData,
                    }
                } else {
                    p
                }
            })
        })
    }
}

impl Node<Audio, Volume> {
    // pub fn volume(&self) -> f32 {
    //     todo!()
    // }

    // pub fn set_volume(&self, volume: f32) {
    //     todo!()
    // }
}

impl NodeHandle<'_, Audio, Volume> {
    pub fn volume(&self) -> f32 {
        todo!()
    }

    pub fn set_volume(&self, volume: f32) {
        todo!()
    }
}

impl Node<Audio, AudioSource> {
    // TODO: seeking
    // fn seek_to()
    // fn seek_by()
    // fn position()

    pub fn stream<'a>(
        &'a mut self,
    ) -> impl Stream<Item = Result<Packet<Audio>, VoiceError>> + Send + 'a {
        // TODO: don't panic
        let track_id = self.kind.track.id;
        let base = self.kind.track.codec_params.time_base.unwrap();
        let mut interval = tokio::time::interval(Duration::from_millis(20));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        stream::unfold((self, interval), move |(state, mut interval)| async move {
            loop {
                let packet = match state.format.next_packet() {
                    Ok(packet) => packet,
                    Err(symphonia::core::errors::Error::IoError(e))
                        if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                    {
                        return None;
                    }
                    Err(e) => {
                        return Some((Err(VoiceError::from(e)), (state, interval)));
                    }
                };

                if packet.track_id() == track_id {
                    let time = MediaTime::new(packet.ts(), Frequency::new(base.denom).unwrap());
                    // PERF: don't clone
                    let data = packet.data.clone();
                    let p = Packet {
                        data,
                        time,
                        _kind: Audio,
                    };
                    interval.tick().await;
                    return Some((Ok(p), (state, interval)));
                }

                // packet didn't match, go around the loop again
            }
        })
    }
}
