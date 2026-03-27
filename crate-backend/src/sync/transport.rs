use std::collections::VecDeque;

use async_trait::async_trait;
use axum::extract::ws::WebSocket;
use common::v1::types::{MessageClient, MessageEnvelope, SyncFormat};
use flate2::{Compress, Decompress, FlushCompress, FlushDecompress};
use futures::{
    stream::{self, BoxStream, SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use lamprey_backend_core::prelude::*;

use crate::sync::WsMessage;

/// how this connection is sending messages to and receiving from the client
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    fn split(self) -> (Box<dyn TransportSink>, TransportStream);
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

/// WebSocket close codes that indicate a clean/normal closure
fn is_clean_close(code: u16) -> bool {
    // 1000 = Normal Closure, 1001 = Going Away
    code == 1000 || code == 1001
}

/// a websocket based transport
pub struct WebsocketTransport {
    sink: SplitSink<WebSocket, WsMessage>,
    stream: SplitStream<WebSocket>,
    format: SyncFormat,
    compression: Option<Compression>,
    inbox: VecDeque<MessageClient>,
}

pub enum Compression {
    Deflate {
        compressor: Compress,
        decompressor: Decompress,
        buffer: Vec<u8>,
    },
}

impl WebsocketTransport {
    pub fn new(ws: WebSocket, format: SyncFormat, use_deflate: bool) -> Self {
        let (sink, stream) = ws.split();
        let compression = if use_deflate {
            Some(Compression::Deflate {
                compressor: Compress::new(flate2::Compression::default(), true),
                decompressor: Decompress::new(true),
                buffer: Vec::with_capacity(4096),
            })
        } else {
            None
        };

        Self {
            sink,
            stream,
            format,
            compression,
            inbox: VecDeque::new(),
        }
    }
}

pub struct WebsocketSink {
    sink: SplitSink<WebSocket, WsMessage>,
    format: SyncFormat,
    compressor: Option<Compress>,
}

pub struct WebsocketReceiver {
    stream: SplitStream<WebSocket>,
    format: SyncFormat,
    decompressor: Option<(Decompress, Vec<u8>)>,
    inbox: VecDeque<MessageClient>,
}

#[async_trait]
impl TransportSink for WebsocketSink {
    async fn send(&mut self, envelope: MessageEnvelope) -> Result<()> {
        let bytes = match self.format {
            SyncFormat::Msgpack => rmp_serde::to_vec_named(&envelope)?,
            SyncFormat::Json => serde_json::to_vec(&envelope)?,
        };

        let final_msg = if let Some(compressor) = &mut self.compressor {
            let mut output = Vec::with_capacity(bytes.len() + 64);
            let total_in = compressor.total_in() as usize;

            loop {
                if output.capacity() - output.len() < 1024 {
                    output.reserve(1024);
                }
                let consumed = (compressor.total_in() as usize) - total_in;
                match compressor.compress_vec(
                    &bytes[consumed..],
                    &mut output,
                    FlushCompress::Sync,
                )? {
                    flate2::Status::StreamEnd => break,
                    flate2::Status::BufError => {}
                    flate2::Status::Ok => {
                        if (compressor.total_in() as usize) - total_in == bytes.len() {
                            break;
                        }
                    }
                }
            }
            WsMessage::binary(output)
        } else {
            match self.format {
                SyncFormat::Msgpack => WsMessage::binary(bytes),
                SyncFormat::Json => {
                    let s =
                        String::from_utf8(bytes).map_err(|e| Error::BadRequest(e.to_string()))?;
                    WsMessage::text(s)
                }
            }
        };

        self.sink.send(final_msg).await.map_err(Into::into)
    }

    async fn close(&mut self) -> Result<()> {
        self.sink.close().await.map_err(Into::into)
    }
}

impl WebsocketReceiver {
    fn into_stream(self) -> TransportStream {
        Box::pin(stream::unfold(self, |mut recv| async move {
            if let Some(msg) = recv.inbox.pop_front() {
                return Some((Ok(TransportEvent::Message(msg)), recv));
            }

            loop {
                let raw = recv.stream.next().await?;

                let bytes = match raw {
                    Err(e) => return Some((Err(e.into()), recv)),
                    Ok(WsMessage::Text(s)) => {
                        let result = serde_json::from_str(&s)
                            .map(TransportEvent::Message)
                            .map_err(Into::into);
                        return Some((result, recv));
                    }
                    Ok(WsMessage::Binary(b)) => b,
                    Ok(WsMessage::Ping(_)) | Ok(WsMessage::Pong(_)) => continue,
                    Ok(WsMessage::Close(close_frame)) => {
                        let clean = close_frame
                            .as_ref()
                            .map(|cf| is_clean_close(cf.code.into()))
                            .unwrap_or(true);
                        return Some((Ok(TransportEvent::Closed(clean)), recv));
                    }
                };

                if let Some((decompressor, buffer)) = &mut recv.decompressor {
                    let mut input_offset = 0;
                    let mut output = [0u8; 4096];
                    while input_offset < bytes.len() {
                        let before_in = decompressor.total_in();
                        let before_out = decompressor.total_out();

                        let status = match decompressor.decompress(
                            &bytes[input_offset..],
                            &mut output,
                            FlushDecompress::None,
                        ) {
                            Ok(s) => s,
                            Err(e) => return Some((Err(e.into()), recv)),
                        };

                        let consumed = (decompressor.total_in() - before_in) as usize;
                        let produced = (decompressor.total_out() - before_out) as usize;
                        buffer.extend_from_slice(&output[..produced]);
                        input_offset += consumed;

                        if status == flate2::Status::StreamEnd || (consumed == 0 && produced == 0) {
                            break;
                        }
                    }

                    let mut consumed = 0;
                    let mut iter =
                        serde_json::Deserializer::from_slice(buffer).into_iter::<MessageClient>();
                    while let Some(msg_res) = iter.next() {
                        match msg_res {
                            Ok(msg) => {
                                recv.inbox.push_back(msg);
                                consumed = iter.byte_offset();
                            }
                            Err(e) if e.is_eof() => break,
                            Err(e) => return Some((Err(e.into()), recv)),
                        }
                    }
                    // reborrow after iter drop
                    let (_, buffer) = recv.decompressor.as_mut().unwrap();
                    if consumed > 0 {
                        buffer.drain(..consumed);
                    }

                    if let Some(msg) = recv.inbox.pop_front() {
                        return Some((Ok(TransportEvent::Message(msg)), recv));
                    }
                } else if recv.format == SyncFormat::Msgpack {
                    let result = rmp_serde::from_slice::<MessageClient>(&bytes)
                        .map(TransportEvent::Message)
                        .map_err(Into::into);
                    return Some((result, recv));
                } else {
                    return Some((
                        Err(Error::BadStatic(
                            "unexpected binary message for uncompressed session",
                        )),
                        recv,
                    ));
                }
            }
        }))
    }
}

impl Transport for WebsocketTransport {
    fn split(self) -> (Box<dyn TransportSink>, TransportStream) {
        let (sink, stream) = (self.sink, self.stream);
        let (compressor, decompressor) = match self.compression {
            Some(Compression::Deflate {
                compressor,
                decompressor,
                buffer,
            }) => (Some(compressor), Some((decompressor, buffer))),
            None => (None, None),
        };

        let sink = Box::new(WebsocketSink {
            sink,
            format: self.format,
            compressor,
        });

        let receiver = WebsocketReceiver {
            stream,
            format: self.format,
            decompressor,
            inbox: self.inbox,
        };

        (sink, receiver.into_stream())
    }
}
