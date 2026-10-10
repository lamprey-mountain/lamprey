use std::time::Duration;

use crate::prelude::*;
use common::{
    v1::types::{
        ChannelType, MessageClient, MessageEnvelope, MessagePayload, SyncParams, SyncVersion,
        error::SyncErrorCode,
    },
    v2::types::sync::stream::StreamHeader,
};
use kerosene_core::types::documents::EditContextId;
use kerosene_services::services::connections::ConnectionHandle;
use kerosene_sync::transport::{
    Compression, Transport, TransportEvent, TransportStream, WebtransportTransport,
    WrapperTransport,
};
use tokio::{spawn, sync::Mutex, task::JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use wtransport::{
    Connection, Endpoint, RecvStream, SendStream, ServerConfig, endpoint::IncomingSession,
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
        let srv = self.globals.services();
        let Some(wt_config) = &self.globals.config().webtransport else {
            info!("webtransport disabled");
            return Ok(());
        };

        info!("starting webtransport server on port {}", wt_config.port);

        // TODO: allow configuring what the wt server should bind to (change port, ip addr)
        let identity = srv.config.webtransport_identity();
        let config = ServerConfig::builder()
            .with_bind_default(wt_config.port)
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

    match params.version {
        SyncVersion::V1 => {
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
        SyncVersion::V2 => v2::accept(globals, connection, params).await,
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

mod v2 {
    use super::*;
    use crate::prelude::*;

    use common::v1::types::Session;
    use common::v1::types::presence::Presence;
    use common::v2::types::ConnectionId;
    use common::v2::types::sync::stream::hello;
    use common::v2::types::sync::transport::webtransport::Params;
    use futures::FutureExt;
    use futures_util::future::Shared;
    use kerosene_sync::v2::codec::Codec;
    use kerosene_sync::v2::transport::webtransport::{WebtransportStream, WebtransportTransport};
    use kerosene_sync::v2::transport::{Transport, TransportStream};
    use tokio::io::AsyncReadExt;
    use tokio::sync::oneshot;

    const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

    #[derive(Clone)]
    struct State {
        globals: Globals,
        id: ConnectionId,
        params: Params,
        session: Shared<oneshot::Receiver<Arc<Session>>>,
    }

    impl State {
        pub async fn session(&self) -> Result<Arc<Session>> {
            self.session
                .clone()
                .await
                .map_err(|_| Error::BadStatic("transport closed"))
        }
    }

    pub async fn accept(
        globals: Globals,
        connection: Connection,
        params: SyncParams,
    ) -> Result<()> {
        let (session_tx, session_rx) = oneshot::channel();
        let state = State {
            globals,
            id: ConnectionId::new(),
            params: Params {
                version: params.version,
                compression: params.compression,
                encoding: params.format,
            },
            session: session_rx.shared(),
        };

        let mut transport = WebtransportTransport::new(connection);
        let mut stream_tasks = JoinSet::new();

        // TODO: use proper sync error, close stream on auth fail
        // TODO: timeout HANDSHAKE_TIMEOUT, close transport if no hello was received in time

        // let (session, hello_frames) =
        //     tokio::time::timeout(HANDSHAKE_TIMEOUT, handshake(&mut transport, shared.clone()))
        //         .await
        //         .map_err(|_| Error::BadStatic("handshake timeout"))??;

        // handle streams
        while let Some(s) = transport.next().await {
            let state = state.clone();
            stream_tasks.spawn(async move {
                if let Err(err) = handle_stream(s, state).await {
                    debug!("error while handling stream: {err}");
                }
            });
        }

        // TODO: somehow get connection closed reason here?
        // debug!("connection closed: {reason}");

        // TODO: clean up connection?

        // TODO: use proper close code (maybe make close() a part of Transport)
        // transport
        //     .into_inner()
        //     .close(SyncErrorCode::Unauthorized, &[]);

        Ok(())
    }

    async fn handle_stream<S: TransportStream>(mut stream: S, state: State) -> Result<()> {
        let srv = state.globals.services();

        // read stream header
        let header = StreamHeader::try_from(stream.read_u8().await?).map_err(
            // TODO: better error types? (eg. dedicated sync error type)
            |_| ApiError::with_message(ErrorCode::InvalidData, "unknown header byte".to_string()),
        )?;

        match header {
            StreamHeader::Hello => {
                type Proto = hello::Protocol;
                let codec = Codec::<Proto>::new(state.params.clone());
                let (init, framed) = codec.accept(stream).await?;
                match init {
                    hello::Initial::Identify(identify) => {
                        // authenticate session
                        let session = srv.sessions.get_by_token(identify.token).await?;

                        // setup presence
                        if let (presence, Some(user_id)) = (identify.presence, session.user_id()) {
                            let user = srv.users.get(user_id, Some(user_id)).await?;
                            if !user.is_suspended() {
                                let presence = presence.unwrap_or_else(Presence::online);
                                srv.presence.set(state.id, user_id, presence);
                            }
                        }

                        // FIXME: session_tx.send(session)

                        Ok(())
                    }
                    // TODO: handle these later
                    // hello::Initial::Resume(resume) => todo!(),
                    // hello::Initial::Shard(shard) => todo!(),
                    _ => Err(Error::Unimplemented),
                }
            }
            _ => {
                let _session = state.session().await?;
                // TODO: do stuff with session
                Err(Error::Unimplemented)
            }
        }
    }
}
