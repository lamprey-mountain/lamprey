// TODO: share this module with lamprey-sdk (maybe put in common?)

use std::collections::VecDeque;

use async_trait::async_trait;
use axum::extract::ws::WebSocket;
use flate2::{Compress, Decompress, FlushCompress, FlushDecompress};
use futures::{
    SinkExt, StreamExt,
    stream::{self, BoxStream, SplitSink, SplitStream},
};
use lamprey::v1::types::{MessageClient, MessageEnvelope, SyncFormat, SyncParams};

use crate::prelude::*;

/// how this connection is sending messages to and receiving from the client
#[async_trait]
pub trait Transport: Send + 'static {
    fn split(self: Box<Self>) -> (Box<dyn TransportSink>, TransportStream);
}

pub type AnyTransport = Box<dyn Transport>;

/// trait for sending messages to a transport
#[async_trait]
pub trait TransportSink: Send + Sync + 'static {
    async fn send(&mut self, msg: MessageEnvelope) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
}

/// trait for receiving messages from a transport
pub type TransportStream = BoxStream<'static, Result<TransportEvent>>;

/// an event returned from a Transport
pub enum TransportEvent {
    /// recv message from transport
    Message(MessageClient),

    /// this transport was closed. indicates whether this is a clean close or
    /// not, for cleanup and presence handling.
    Closed(bool),
}

pub enum Compression {
    Deflate {
        compressor: Compress,
        decompressor: Decompress,
        buffer: Vec<u8>,
    },
}

pub use websocket::{WebsocketReceiver, WebsocketSink, WebsocketTransport};
pub use wrapper::WrapperTransport;

mod websocket;
mod wrapper;

#[cfg(feature = "webtransport")]
mod webtransport;

#[cfg(feature = "webtransport")]
pub use webtransport::{WebtransportReceiver, WebtransportSink, WebtransportTransport};
