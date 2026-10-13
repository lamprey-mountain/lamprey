use std::{
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
};

use common::v1::types::voice::{KeyframeRequestKind, MediaKind};
use futures::Stream;
use str0m::media::Rid;

use super::{Frame, Producer};
use crate::voice::VoiceError;

/// switches which stream to pull from based on rid
pub struct Switchboard<P = Box<dyn Producer>> {
    kind: MediaKind,
    layers: HashMap<Rid, P>,
    // TODO: multiple active layers?
    active: Option<Rid>,
}

impl<P: Producer> Switchboard<P> {
    /// create a new switchboard for the given media kind
    pub fn new(kind: MediaKind) -> Self {
        Self {
            kind,
            layers: HashMap::new(),
            active: None,
        }
    }

    /// create a new video switchboard
    pub fn video() -> Self {
        Self::new(MediaKind::Video)
    }

    /// add a simulcast layer producer for the given `rid`
    pub fn layer(mut self, rid: Rid, producer: P) -> Self {
        if self.active.is_none() {
            self.active = Some(rid);
        }
        self.layers.insert(rid, producer);
        self
    }

    /// get the currently active layer `Rid`
    pub fn active_layer(&self) -> Option<Rid> {
        self.active
    }
}

impl<P: Producer> Stream for Switchboard<P> {
    type Item = Result<Frame, VoiceError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(active_rid) = self.active else {
            return Poll::Ready(None);
        };

        let Some(producer) = self.layers.get_mut(&active_rid) else {
            return Poll::Ready(None);
        };

        match Pin::new(producer).poll_next(cx) {
            Poll::Ready(Some(Ok(mut frame))) => {
                frame.rid = Some(active_rid);
                Poll::Ready(Some(Ok(frame)))
            }
            other => other,
        }
    }
}

impl<P: Producer> Producer for Switchboard<P> {
    fn request_keyframe(&mut self, kind: KeyframeRequestKind, rid: Option<Rid>) {
        if let Some(rid) = rid.or(self.active) {
            if let Some(producer) = self.layers.get_mut(&rid) {
                producer.request_keyframe(kind, Some(rid));
            }
        } else {
            for (&layer_rid, producer) in &mut self.layers {
                producer.request_keyframe(kind, Some(layer_rid));
            }
        }
    }

    fn select_layer(&mut self, rid: Rid) {
        if self.layers.contains_key(&rid) {
            self.active = Some(rid);
        }
    }

    fn kind(&self) -> MediaKind {
        self.kind
    }
}
