use std::time::Duration;

use crate::prelude::*;
use common::v1::types::{
    ChannelType, MessageClient, MessageEnvelope, MessagePayload, SyncParams, error::SyncErrorCode,
};
use flate2::{Compress, Decompress, FlushCompress, FlushDecompress};
use kerosene_core::types::documents::EditContextId;
use kerosene_services::services::connections::ConnectionHandle;
use kerosene_sync::transport::{
    Compression, Transport, TransportEvent, TransportStream, WebtransportTransport,
    WrapperTransport,
};
use tokio::{spawn, sync::Mutex, task::JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use wtransport::{Endpoint, RecvStream, SendStream, ServerConfig, endpoint::IncomingSession};

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
        globals,
        params,
        shared: Mutex::new(WtStateShared::default()),
    });

    loop {
        tokio::select! {
            Ok((send, recv)) = connection.accept_bi() => {
                let state = state.clone();
                spawn(handle_stream(send, recv, state));
            }
            reason = connection.closed() => {
                // TODO: handle reason correctly, return Ok or Err depending on it
                debug!("connection closed: {reason}");
                return Ok(())
            }
        }
    }
}

async fn handle_stream(send: SendStream, recv: RecvStream, state: WtState) {
    if let Err(err) = handle_stream_inner(send, recv, state).await {
        debug!("error while handling stream: {err}");
    }
}

// bodged together these types, they could probably be designed better
type WtState = Arc<WtStateInner>;

struct WtStateInner {
    globals: Globals,
    params: SyncParams,
    shared: Mutex<WtStateShared>,
}

#[derive(Debug, Default)]
struct WtStateShared {
    connection: Option<ConnectionHandle>,
}

impl WtStateShared {
    fn ready(&self) -> bool {
        self.connection.is_some()
    }
}

async fn handle_stream_inner(send: SendStream, recv: RecvStream, state: WtState) -> Result<()> {
    let srv = state.globals.services();

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

    // TODO: use better/stricter types instead of reusing MessageClient
    // TODO: error on MessageClient subscriptions from webtransport, require using bidirectional streams to subscribe
    // TODO: refactor this code
    let mut shared = state.shared.lock().await;
    match init {
        // the first message of the first stream MUST be a hello
        // no other streams should be created until this handshake is complete
        MessageClient::Hello(hello) => {
            if let Some(resume) = &hello.resume {
                let handle = shared
                    .connection
                    .as_ref()
                    .ok_or(SyncErrorCode::Unauthenticated);
                let Ok(handle) = handle else {
                    // TODO: better error handling (avoid unwraps)
                    // TODO: create sync error code for expired connection
                    send.send(MessageEnvelope {
                        payload: MessagePayload::Error {
                            error: "expired or invalid connection".into(),
                            code: Some(SyncErrorCode::ConnectionExpired),
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
                shared.connection = Some(handle.clone());
            } else {
                if shared.ready() {
                    send.send(MessageEnvelope {
                        payload: MessagePayload::Error {
                            error: "you already have a sync stream".into(),
                            // NOTE: maybe i should let clients have multiple sync
                            // streams? ie. allow having different priority sync
                            // streams filtered to different events?
                            code: Some(SyncErrorCode::AlreadyAuthenticated),
                        },
                    })
                    .await
                    .unwrap();
                    send.close().await.unwrap();
                    return Ok(());
                }

                let handle = srv.connections.accept(hello).await?;
                let transport = Box::new(WrapperTransport::new(send, recv));
                handle.attach(transport, 0);
                shared.connection = Some(handle.clone());
            };
        }

        MessageClient::DocumentSubscribe {
            channel_id,
            branch_id,
            state_vector,
        } => {
            let handle = shared
                .connection
                .as_ref()
                .ok_or(SyncErrorCode::Unauthenticated);
            let Ok(handle) = handle else {
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

            // HACK: lookup whether this is for a redex
            let chan = srv.channels.get(channel_id, None).await?;
            let is_redex = chan.ty == ChannelType::Scripts;

            let context_id = if is_redex {
                EditContextId::from_redex(channel_id, (*branch_id).into())
            } else {
                EditContextId::from_prose(channel_id, branch_id)
            };
            let transport = Box::new(WrapperTransport::new(send, recv));
            handle.attach_document_transport(context_id, transport, state_vector);
        }

        MessageClient::VoiceConnect { voice_state, nonce } => {
            let handle = shared
                .connection
                .as_ref()
                .ok_or(SyncErrorCode::Unauthenticated);
            let Ok(handle) = handle else {
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

            let transport = Box::new(WrapperTransport::new(send, recv));
            handle.attach_voice(transport, voice_state, nonce);
        }

        MessageClient::MemberListSubscribe {
            room_id,
            thread_id,
            ranges,
        } => {
            let handle = shared
                .connection
                .as_ref()
                .ok_or(SyncErrorCode::Unauthenticated);
            let Ok(handle) = handle else {
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

            let transport = Box::new(WrapperTransport::new(send, recv));
            handle.attach_member_list(transport, room_id, thread_id, ranges);
        }

        MessageClient::ScriptSubscribe {
            channel_id,
            script_id,
        } => {
            let handle = shared
                .connection
                .as_ref()
                .ok_or(SyncErrorCode::Unauthenticated);
            let Ok(handle) = handle else {
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

            let transport = Box::new(WrapperTransport::new(send, recv));
            handle.attach_script(transport, channel_id, script_id);
        }

        MessageClient::RoomSubscribe { room_id } => {
            let handle = shared
                .connection
                .as_ref()
                .ok_or(SyncErrorCode::Unauthenticated);
            let Ok(handle) = handle else {
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

            let transport = Box::new(WrapperTransport::new(send, recv));
            handle.attach_room(transport, room_id);
        }

        MessageClient::ChannelSubscribe { channel_id } => {
            let handle = shared
                .connection
                .as_ref()
                .ok_or(SyncErrorCode::Unauthenticated);
            let Ok(handle) = handle else {
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

            let transport = Box::new(WrapperTransport::new(send, recv));
            handle.attach_channel(transport, channel_id);
        }

        _ => return Err(Error::BadStatic("invalid client message")),
    }

    Ok(())
}
