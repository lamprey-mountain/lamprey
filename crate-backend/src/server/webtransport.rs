use std::{collections::VecDeque, time::Duration};

use crate::prelude::*;
use async_trait::async_trait;
use common::{
    v1::types::{
        MessageClient, MessageEnvelope, MessagePayload, SyncFormat, SyncParams,
        error::SyncErrorCode,
    },
    v2::types::ConnectionId,
};
use flate2::{Compress, Decompress, FlushCompress, FlushDecompress};
use kerosene_core::types::documents::EditContextId;
use kerosene_sync::transport::{
    Compression, Transport, TransportEvent, TransportSink, TransportStream, WebtransportTransport,
    WrapperTransport,
};
use tokio::{io::AsyncWriteExt, spawn, sync::Mutex, task::JoinSet};
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

        // TODO: allow configuring what the wt server should bind to (change port, ip addr)
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
    debug!("accepted session");
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

    let state: WtState = Arc::new(WtStateInner {
        params,
        shared: Mutex::new(WtStateShared::default()),
    });

    loop {
        tokio::select! {
            Ok((send, recv)) = connection.accept_bi() => {
                let globals = globals.clone();
                let state = state.clone();
                spawn(handle_stream(globals, send, recv, state));
            }
            reason = connection.closed() => {
                // TODO: handle reason correctly, return Ok or Err depending on it
                debug!("connection closed: {reason}");
                return Ok(())
            }
        }
    }
}

async fn handle_stream(globals: Globals, send: SendStream, recv: RecvStream, state: WtState) {
    if let Err(err) = handle_stream_inner(globals, send, recv, state).await {
        debug!("error while handling stream: {err}");
    }
}

// bodged together these types, they could probably be designed better
type WtState = Arc<WtStateInner>;

struct WtStateInner {
    params: SyncParams,
    shared: Mutex<WtStateShared>,
}

#[derive(Debug, Default)]
struct WtStateShared {
    connection_id: Option<ConnectionId>,
}

impl WtStateShared {
    fn ready(&self) -> bool {
        self.connection_id.is_some()
    }
}

async fn handle_stream_inner(
    globals: Globals,
    send: SendStream,
    recv: RecvStream,
    state: WtState,
) -> Result<()> {
    let srv = globals.services();

    // PERF: requiring boxing is probably fine but technically slower than it could be
    let transport = Box::new(WebtransportTransport::new(send, recv, state.params.clone()));
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
    let mut shared = state.shared.lock().await;
    match init {
        // the first message of the first stream MUST be a hello
        // no other streams should be created until this handshake is complete
        MessageClient::Hello(hello) => {
            if shared.ready() {
                send.send(MessageEnvelope {
                    payload: MessagePayload::Error {
                        error: "you already have a sync stream".into(),
                        // NOTE: do i reuse AlreadyAuthenticated?
                        // maybe i should let clients have multiple sync
                        // streams? ie. allow having different priority sync
                        // streams filtered to different events?
                        code: None,
                    },
                })
                .await
                .unwrap();
                send.close().await.unwrap();
                return Ok(());
            }

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
                shared.connection_id = Some(handle.id());
            } else {
                let handle = srv.connections.accept(hello).await?;
                let transport = Box::new(WrapperTransport::new(send, recv));
                handle.attach(transport, 0);
                shared.connection_id = Some(handle.id());
            };
        }

        MessageClient::DocumentSubscribe {
            channel_id,
            branch_id,
            state_vector,
        } => {
            let Some(conn_id) = shared.connection_id else {
                send.send(MessageEnvelope {
                    payload: MessagePayload::Error {
                        error: "you need to open and authenticate a sync stream first".into(),
                        code: Some(SyncErrorCode::Unauthenticated),
                    },
                })
                .await
                .unwrap();
                send.close().await.unwrap();
                return Ok(());
            };

            let Some(handle) = srv.connections.get(conn_id) else {
                send.send(MessageEnvelope {
                    payload: MessagePayload::Error {
                        error: "connection is somehow expired, despite the webtransport endpoint still being active?".into(),
                        code: None,
                    },
                })
                .await
                .unwrap();
                send.close().await.unwrap();
                return Ok(());
            };

            let context_id = EditContextId::from_prose(channel_id, branch_id);
            let transport = Box::new(WrapperTransport::new(send, recv));
            handle.attach_document_transport(context_id, transport, state_vector);
        }

        // TODO: subscriptions are replaced with dedicated webtransport streams
        MessageClient::VoiceConnect { .. }
        | MessageClient::MemberListSubscribe { .. }
        | MessageClient::ScriptSubscribe { .. } => return Err(Error::Unimplemented),

        _ => return Err(Error::BadStatic("invalid client message")),
    }

    Ok(())
}
