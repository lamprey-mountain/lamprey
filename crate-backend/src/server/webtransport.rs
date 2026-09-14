use std::{collections::VecDeque, time::Duration};

use crate::prelude::*;
use async_trait::async_trait;
use common::v1::types::{MessageClient, MessageEnvelope, MessagePayload, SyncFormat, SyncParams};
use flate2::{Compress, Decompress, FlushCompress, FlushDecompress};
use kerosene_sync::transport::{
    Compression, Transport, TransportEvent, TransportSink, TransportStream, WrapperTransport,
};
use tokio::{io::AsyncWriteExt, spawn, task::JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use wtransport::{
    Endpoint, Identity, RecvStream, SendStream, ServerConfig,
    endpoint::{IncomingSession, SessionRequest},
};

// TODO: impl better error handling instead of unwrapping everywhere

#[derive(Clone)]
pub struct WtServer {
    globals: Globals,
    token: CancellationToken,
}

impl WtServer {
    pub fn new(globals: Globals) -> Result<Self> {
        Ok(Self {
            globals,
            token: CancellationToken::new(),
        })
    }

    /// get a handle to the server's global state
    pub fn globals(&self) -> Globals {
        self.globals.clone()
    }

    /// start the server
    pub async fn serve(&self) -> Result<()> {
        info!("starting webtransport server");

        let srv = self.globals.services();
        let identity = srv.config.webtransport_identity();
        let config = ServerConfig::builder()
            .with_bind_default(4433)
            .with_identity(identity.clone_identity())
            .build();
        let server = Endpoint::server(config)?;

        // TODO: proper routing
        // let mut router = Router::new();
        // router.insert("/api/v1/sync-webtransport", ()).unwrap();

        let mut sessions = JoinSet::new();

        loop {
            tokio::select! {
                _ = self.token.cancelled() => {
                    break;
                }
                incoming = server.accept() => {
                    sessions.spawn(handle_session(self.globals(), incoming));
                }
                Some(_) = sessions.join_next(), if !sessions.is_empty() => {}
            }
        }

        sessions.shutdown().await;
        Ok(())
    }

    /// cleanly shutdown this server
    pub async fn shutdown(&mut self) -> Result<()> {
        self.token.cancel();
        Ok(())
    }
}

#[tracing::instrument(skip(globals, incoming))]
async fn handle_session(globals: Globals, incoming: IncomingSession) {
    if let Err(err) = handle_session_inner(globals, incoming).await {
        debug!("error while handling session: {err}");
    }
}

async fn handle_session_inner(globals: Globals, incoming: IncomingSession) -> Result<()> {
    let req = incoming.await.unwrap();

    let uri: http::uri::PathAndQuery = req.path().parse().unwrap();
    if uri.path() != "/api/v1/sync-webtransport" {
        req.not_found().await;
        return Ok(());
    }

    let params: SyncParams = serde_urlencoded::from_str(uri.query().unwrap_or_default()).unwrap();
    let connection = req.accept().await.unwrap();

    loop {
        tokio::select! {
            Ok((send, recv)) = connection.accept_bi() => {
                let globals = globals.clone();
                let params = params.clone();
                spawn(handle_stream(globals, send, recv, params));
            }
            reason = connection.closed() => {
                // TODO: handle reason correctly, return Ok or Err depending on it
                debug!("connection closed: {reason}");
                return Ok(())
            }
        }
    }
}

async fn handle_stream(globals: Globals, send: SendStream, recv: RecvStream, params: SyncParams) {
    if let Err(err) = handle_stream_inner(globals, send, recv, params).await {
        debug!("error while handling stream: {err}");
    }
}

async fn handle_stream_inner(
    globals: Globals,
    send: SendStream,
    recv: RecvStream,
    params: SyncParams,
) -> Result<()> {
    let srv = globals.services();

    // PERF: requiring boxing is probably fine but technically slower than it could be
    let transport = Box::new(WebtransportTransport::new(send, recv, params));
    let (mut send, mut recv) = transport.split();

    // NOTE: do i need to set up a minimal impl of the protocol here?
    let init = tokio::time::timeout(Duration::from_secs(5), recv.next()).await;

    // outer result: tokio timeout
    // option: client not sending any more messages
    // inner result: transport errors
    let Ok(Some(Ok(TransportEvent::Message(init)))) = init else {
        // TODO: better error handling
        send.close().await.unwrap();
        return Ok(());
    };

    let srv = globals.services();

    // TODO: use better/stricter types instead of reusing MessageClient
    // TODO: error on MessageClient subscriptions from webtransport, require using bidirectional streams to subscribe
    match init {
        // the first message of the first stream MUST be a hello
        // no other streams should be created until this handshake is complete
        MessageClient::Hello(hello) => {
            if let Some(resume) = &hello.resume {
                let Some(handle) = srv.connections.get(resume.conn) else {
                    // TODO: better error handling (avoid unwraps)
                    // TODO: create sync error code for expired connection
                    send.send(MessageEnvelope {
                        payload: MessagePayload::Error {
                            error: "expired or invalid connection".into(),
                            code: None,
                        },
                    })
                    .await
                    .unwrap();
                    send.send(MessageEnvelope {
                        payload: MessagePayload::Reconnect { can_resume: false },
                    })
                    .await
                    .unwrap();
                    send.close().await.unwrap();
                    return Ok(());
                };

                let transport = Box::new(WrapperTransport::new(send, recv));
                handle.attach(transport, resume.seq);
            } else {
                let handle = srv.connections.accept(hello).await?;
                let transport = Box::new(WrapperTransport::new(send, recv));
                handle.attach(transport, 0);
            };
        }

        // TODO: subscriptions are replaced with dedicated webtransport streams
        MessageClient::VoiceConnect { .. }
        | MessageClient::MemberListSubscribe { .. }
        | MessageClient::DocumentSubscribe { .. }
        | MessageClient::ScriptSubscribe { .. } => return Err(Error::Unimplemented),

        _ => return Err(Error::BadStatic("invalid client message")),
    }

    Ok(())
}

// TODO: move below to kerosene-sync

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
        Box::pin(futures::stream::unfold(self, |mut me| async move {
            if let Some(msg) = me.inbox.pop_front() {
                return Some((Ok(TransportEvent::Message(msg)), me));
            }

            // read length
            let mut len_buf = [0u8; 4];
            me.recv.read_exact(&mut len_buf).await.unwrap();
            let len = u32::from_be_bytes(len_buf) as usize;

            // TODO: what should the max length be? (currently 1mb)
            if len > 1 * 1024 * 1024 {
                return Some((Err(Error::BadStatic("webtransport frame too large")), me));
            }

            // read payload
            let mut payload = vec![0u8; len];
            // NOTE: probably should return TransportEvent::Closed(false)
            me.recv.read_exact(&mut payload).await.unwrap();

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
