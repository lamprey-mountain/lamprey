use std::collections::VecDeque;

use async_trait::async_trait;
use flate2::{Compress, Decompress, FlushCompress, FlushDecompress};
use futures::stream;
use lamprey::v1::types::{MessageClient, MessageEnvelope, SyncFormat, SyncParams};
use tokio::io::AsyncWriteExt;
use wtransport::{RecvStream, SendStream};

use crate::prelude::*;
use crate::transport::{Compression, Transport, TransportEvent, TransportSink, TransportStream};

/// a webtransport based transport
// each message is framed with a u32 big endian length prefix
// NOTE: do i need to frame websocket messages too?
pub struct WebtransportTransport {
    send: SendStream,
    recv: RecvStream,
    format: SyncFormat,
    compression: Option<Compression>,
    inbox: VecDeque<MessageClient>,
}

pub struct WebtransportSink {
    send: SendStream,
    format: SyncFormat,
    compressor: Option<Compress>,
}

pub struct WebtransportReceiver {
    recv: RecvStream,
    format: SyncFormat,
    decompressor: Option<(Decompress, Vec<u8>)>,
    inbox: VecDeque<MessageClient>,
}

impl WebtransportTransport {
    pub fn new(send: SendStream, recv: RecvStream, params: SyncParams) -> Self {
        let compression = if params.compression.is_some() {
            Some(Compression::Deflate {
                compressor: Compress::new(flate2::Compression::default(), true),
                decompressor: Decompress::new(true),
                buffer: Vec::with_capacity(4096),
            })
        } else {
            None
        };

        Self {
            send,
            recv,
            format: params.format,
            compression,
            inbox: VecDeque::new(),
        }
    }
}

impl Transport for WebtransportTransport {
    fn split(self: Box<Self>) -> (Box<dyn TransportSink>, TransportStream) {
        let (compressor, decompressor) = match self.compression {
            Some(Compression::Deflate {
                compressor,
                decompressor,
                buffer,
            }) => (Some(compressor), Some((decompressor, buffer))),
            None => (None, None),
        };

        let sink = Box::new(WebtransportSink {
            send: self.send,
            format: self.format,
            compressor,
        });

        let receiver = WebtransportReceiver {
            recv: self.recv,
            format: self.format,
            decompressor,
            inbox: self.inbox,
        };

        (sink, receiver.into_stream())
    }
}

#[async_trait]
impl TransportSink for WebtransportSink {
    async fn send(&mut self, envelope: MessageEnvelope) -> Result<()> {
        let bytes = match self.format {
            SyncFormat::Msgpack => rmp_serde::to_vec_named(&envelope)?,
            SyncFormat::Json => serde_json::to_vec(&envelope)?,
        };

        // unlike websocket, all messages are sent as binary
        let payload = if let Some(compressor) = &mut self.compressor {
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
            output
        } else {
            bytes
        };

        let len =
            u32::try_from(payload.len()).map_err(|_| Error::BadStatic("message too large"))?;
        self.send
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| Error::Internal(format!("webtransport write error: {e}")))?;
        self.send
            .write_all(&payload)
            .await
            .map_err(|e| Error::Internal(format!("webtransport write error: {e}")))?;
        // PERF: split flush() into its own method so that the connection actor can batch writes
        // or make send() take multiple MessageEnvelopes
        self.send
            .flush()
            .await
            .map_err(|e| Error::Internal(format!("webtransport write error: {e}")))?;

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.send
            .finish()
            .await
            .map_err(|e| Error::Internal(format!("webtransport finish error: {e}")))?;
        Ok(())
    }
}

impl WebtransportReceiver {
    fn into_stream(self) -> TransportStream {
        Box::pin(stream::unfold(self, |mut me| async move {
            if let Some(msg) = me.inbox.pop_front() {
                return Some((Ok(TransportEvent::Message(msg)), me));
            }

            // read length
            let mut len_buf = [0u8; 4];
            if let Err(_) = me.recv.read_exact(&mut len_buf).await {
                return None;
            }
            let len = u32::from_be_bytes(len_buf) as usize;

            // TODO: what should the max length be? (currently 1mb)
            if len > 1 * 1024 * 1024 {
                return Some((Err(Error::BadStatic("webtransport frame too large")), me));
            }

            // read payload
            let mut payload = vec![0u8; len];
            // NOTE: probably should return TransportEvent::Closed(false)
            if let Err(_) = me.recv.read_exact(&mut payload).await {
                return None;
            }

            if let Some((decompressor, buffer)) = &mut me.decompressor {
                let mut input_offset = 0;
                let mut output = [0u8; 4096];
                while input_offset < payload.len() {
                    let before_in = decompressor.total_in();
                    let before_out = decompressor.total_out();

                    let status = match decompressor.decompress(
                        &payload[input_offset..],
                        &mut output,
                        FlushDecompress::None,
                    ) {
                        Ok(s) => s,
                        Err(e) => return Some((Err(e.into()), me)),
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
                            me.inbox.push_back(msg);
                            consumed = iter.byte_offset();
                        }
                        Err(e) if e.is_eof() => break,
                        Err(e) => return Some((Err(e.into()), me)),
                    }
                }
                // reborrow after iter drop
                let (_, buffer) = me.decompressor.as_mut().unwrap();
                if consumed > 0 {
                    buffer.drain(..consumed);
                }

                if let Some(msg) = me.inbox.pop_front() {
                    return Some((Ok(TransportEvent::Message(msg)), me));
                }
            } else {
                let result = match me.format {
                    SyncFormat::Msgpack => rmp_serde::from_slice::<MessageClient>(&payload)
                        .map(TransportEvent::Message)
                        .map_err(Into::into),
                    SyncFormat::Json => serde_json::from_slice::<MessageClient>(&payload)
                        .map(TransportEvent::Message)
                        .map_err(Into::into),
                };
                return Some((result, me));
            }

            Some((
                Err(Error::BadStatic(
                    "webtransport: decompressed frame contained no complete messages",
                )),
                me,
            ))
        }))
    }
}
