use std::{future::Future, sync::Arc};

use common::{
    v1::types::voice::{MediaKind, Mid, TrackKey},
    v2::types::UserId,
};
use futures_util::{StreamExt, stream::BoxStream};

use crate::voice::{VoiceError, peer::ConnectionState};

/// sending data
pub struct Outbound {
    state: Arc<ConnectionState>,
}

/// negotiating, will result in Outbound
pub struct OutboundPending {
    state: Arc<ConnectionState>,
}

/// receiving data
pub struct Inbound {
    state: Arc<ConnectionState>,
}

impl Outbound {
    pub fn mid(&self) -> Mid {
        todo!()
    }

    pub fn kind(&self) -> MediaKind {
        todo!()
    }

    pub fn key(&self) -> TrackKey {
        todo!()
    }

    /// stop and remove this track
    pub async fn stop(&self) -> Result<(), VoiceError> {
        todo!()
    }
}

impl Inbound {
    pub fn user_id(&self) -> UserId {
        todo!()
    }

    pub fn mid(&self) -> Mid {
        todo!()
    }

    pub fn kind(&self) -> MediaKind {
        todo!()
    }

    pub fn key(&self) -> TrackKey {
        todo!()
    }

    /// attemp to unsubscribe to this track
    pub async fn unsubscribe(&self) -> Result<(), VoiceError> {
        todo!()
    }

    /// stream media from this track
    pub fn stream(&self) -> BoxStream<'static, ()> {
        futures_util::stream::empty().boxed()
    }
}

impl OutboundPending {
    pub fn mid(&self) -> Mid {
        todo!()
    }

    pub fn kind(&self) -> MediaKind {
        todo!()
    }

    pub fn key(&self) -> TrackKey {
        todo!()
    }

    /// stop and remove this track
    pub async fn stop(&self) -> Result<(), VoiceError> {
        todo!()
    }
}

impl Future for OutboundPending {
    type Output = Outbound;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!()
    }
}
