use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[async_trait]
pub trait TransportV2<C, E>: Send + 'static {
    fn split(self: Box<Self>) -> (Box<dyn TransportSinkV2<E>>, TransportStreamV2<C>);
}

#[async_trait]
pub trait TransportSinkV2<E>: Send + Sync + 'static {
    async fn send(&mut self, event: E) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
}

pub type TransportStreamV2<C> = BoxStream<'static, Result<TransportEventV2<C>>>;

pub enum TransportEventV2<C> {
    Message(C),
    Closed(bool),
}

pub mod webtransport;
