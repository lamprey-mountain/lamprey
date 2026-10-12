use std::{future::Future, marker::PhantomData, sync::Arc};

use common::{
    v1::types::voice::{MediaKind, Mid, TrackId, TrackKey},
    v2::types::UserId,
};
use futures_util::{StreamExt, stream::BoxStream};

use crate::voice::{VoiceError, client::VoiceInner};

pub struct Track<K> {
    state: Arc<VoiceInner>,
    _kind: PhantomData<K>,
}

pub struct InboundActive;
pub struct InboundPending;
pub struct OutboundActive;
pub struct OutboundPending;

pub trait Inbound {}
pub trait Outbound {}
pub trait Active {}
pub trait Pending {}

impl Inbound for InboundActive {}
impl Inbound for InboundPending {}
impl Outbound for OutboundActive {}
impl Outbound for OutboundPending {}
impl Active for InboundActive {}
impl Active for OutboundActive {}
impl Pending for InboundPending {}
impl Pending for OutboundPending {}

impl<K> Track<K> {
    /// local mid
    pub fn mid(&self) -> Mid {
        todo!()
    }

    pub fn kind(&self) -> MediaKind {
        todo!()
    }

    pub fn key(&self) -> TrackKey {
        todo!()
    }

    pub fn user_id(&self) -> UserId {
        todo!()
    }

    // /// stop and remove this track
    // // NOTE: maybe merge stop/cancel/unsubscribe here
    // pub async fn stop(&self) -> Result<(), VoiceError> {
    //     todo!()
    // }
}

impl<K: Inbound> Track<K> {
    /// get the assigned track id
    pub fn track_id(&self) -> TrackId {
        todo!()
    }

    /// attempt to unsubscribe to this track
    pub async fn unsubscribe(&self) -> Result<(), VoiceError> {
        todo!()
    }
}

impl Track<InboundActive> {
    /// stream media from this track
    pub fn stream(&self) -> BoxStream<'static, ()> {
        futures_util::stream::empty().boxed()
    }
}

impl Track<OutboundActive> {
    /// get the assigned track id
    pub fn track_id(&self) -> TrackId {
        todo!()
    }

    pub async fn stop(&self) -> Result<(), VoiceError> {
        todo!()
    }
}

impl Track<OutboundPending> {
    pub async fn cancel(self) -> Result<(), VoiceError> {
        todo!()
    }
}

impl Future for Track<OutboundPending> {
    type Output = Track<OutboundActive>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!()
    }
}
