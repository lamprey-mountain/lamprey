use super::*;
use common::v2::types::sync::dispatch::channel::{
    ChannelCreate, ChannelUpdate, DispatchChannel, MessageCreate,
};
use common::v2::types::sync::dispatch::global::DispatchGlobal;
use common::v2::types::sync::dispatch::room::RoomCreate;
use common::v2::types::sync::dispatch::{Dispatch, Ready};
use futures::SinkExt;
use kerosene_services::globals::messaging::Broadcast;
use kerosene_sync::util::Timeout;
use tokio::sync::watch;

use common::v1::types::presence::Presence;
use common::v1::types::{MessageSync, Session};
use common::v2::types::ConnectionId;
use common::v2::types::sync::stream::hello;
use common::v2::types::sync::transport::webtransport::Params;
use kerosene_sync::v2::codec::Codec;
use kerosene_sync::v2::transport::TransportStream;
use kerosene_sync::v2::transport::webtransport::WebtransportTransport;
use tokio::io::AsyncReadExt;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);

struct StateInner {
    globals: Globals,
    id: ConnectionId,
    params: Params,
    session: watch::Sender<Option<Arc<Session>>>,
}

type State = Arc<StateInner>;

impl StateInner {
    pub async fn session(&self) -> Result<Arc<Session>> {
        let mut rx = self.session.subscribe();
        // Wait until a session is set
        loop {
            if let Some(session) = rx.borrow().clone() {
                return Ok(session);
            }
            if rx.changed().await.is_err() {
                return Err(Error::BadStatic("transport closed"));
            }
        }
    }
}

pub async fn accept(
    globals: Globals,
    connection: Connection,
    params: SyncParams,
) -> Result<()> {
    let (session_tx, _) = watch::channel(None);
    let state = Arc::new(StateInner {
        globals,
        id: ConnectionId::new(),
        params: Params {
            version: params.version,
            compression: params.compression,
            encoding: params.format,
        },
        session: session_tx,
    });

    let mut transport = WebtransportTransport::new(connection);
    let mut stream_tasks = JoinSet::new();

    // try to authenticate the client
    let mut timeout = Box::pin(tokio::time::sleep(HANDSHAKE_TIMEOUT));
    loop {
        tokio::select! {
            result = state.session() => {
                result?;
                break;
            }
            _ = &mut timeout => {
                // took too long without authenticating, close the connection
                debug!("handshake timed out before identify; disconnecting");
                transport.into_inner().close(
                    wtransport::VarInt::from_u32(SyncErrorCode::AuthTimeout.code() as u32),
                    b"handshake timeout",
                );
                return Ok(());
            }
            s = transport.next() => {
                let Some(s) = s else {
                    // connection closed
                    return Ok(());
                };

                let state = state.clone();
                stream_tasks.spawn(async move {
                    if let Err(err) = handle_stream(s, state).await {
                        debug!("error while handling stream: {err}");
                    }
                });
            }
        }
    }

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
            let (init, mut framed) = codec.accept(stream).await?;
            match init {
                hello::Initial::Identify(identify) => {
                    // authenticate session
                    let session = srv.sessions.get_by_token(identify.token).await?;
                    let _ = state.session.send(Some(session.clone()));

                    // set presence
                    if let (presence, Some(user_id)) = (identify.presence, session.user_id()) {
                        let user = srv.users.get(user_id, Some(user_id)).await?;
                        if !user.is_suspended() {
                            let presence = presence.unwrap_or_else(Presence::online);
                            srv.presence.set(state.id, user_id, presence);
                        }
                    }

                    // send ready payload
                    let user = if let Some(uid) = session.user_id() {
                        Some(Box::new(srv.users.get(uid, Some(uid)).await?))
                    } else {
                        None
                    };

                    let application = if let Some(application_id) = session.app_id {
                        let mut d = state.globals.begin_read().await?;
                        Some(Box::new(d.application_get(application_id).await?))
                    } else {
                        None
                    };

                    framed
                        .send(hello::Event::Ready(Ready {
                            user,
                            application,
                            session: Box::new((*session).clone()),
                            conn: state.id,
                        }))
                        .await?;

                    // start main loop
                    let mut timeout = Timeout::for_ping();
                    let mut broadcast_stream = state.globals.messaging().subscribe().await?;
                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep_until(timeout.get_instant()) => match timeout {
                                Timeout::Ping(_) => {
                                    framed.send(hello::Event::Ping).await?;
                                    timeout = Timeout::for_close();
                                }
                                Timeout::Close(_) => {
                                    return Err(Error::BadStatic("ping timeout"));
                                }
                            },
                            broadcast = broadcast_stream.next() => {
                                if let Some(msg) = broadcast {
                                    if let Some(msg) = broadcast_to_dispatch(msg) {
                                        framed.send(hello::Event::Dispatch(hello::Dispatch {
                                            inner: Box::new(msg),
                                            seq: 0, // TODO: populate seq
                                        })).await?;
                                    }
                                }
                            },
                            command = framed.next() => {
                                match command.transpose()? {
                                    Some(hello::Command::Pong) => {
                                        timeout = Timeout::for_ping();
                                    }
                                    Some(hello::Command::Presence { presence }) => {
                                        if let Some(user_id) = session.user_id() {
                                            srv.presence.set(state.id, user_id, presence);
                                        }
                                    }
                                    Some(hello::Command::Close) | None => {
                                        break;
                                    }
                                }
                            },
                        }
                    }

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

// TODO: implement this, slowly and painfully
fn broadcast_to_dispatch(broadcast: Broadcast) -> Option<Dispatch> {
    let Broadcast::Sync(sync) = broadcast else {
        return None;
    };

    let dispatch: Dispatch = match sync.message {
        MessageSync::RoomCreate { room } => DispatchGlobal::RoomCreate(RoomCreate {
            room: Box::new(room),
            roles: todo!(),
            channels: todo!(),
            threads: todo!(),
            room_member: todo!(),
            seq: todo!(),
        })
        .into(),
        MessageSync::ChannelCreate { channel } => {
            let seq = channel.latest_seq;
            DispatchGlobal::ChannelCreate(ChannelCreate { channel, seq }).into()
        }
        MessageSync::ChannelUpdate { channel } => {
            let seq = channel.latest_seq;
            DispatchGlobal::ChannelUpdate(ChannelUpdate { channel, seq }).into()
        }
        MessageSync::MessageCreate { message } => {
            DispatchChannel::MessageCreate(MessageCreate {
                message: Box::new(message),
                seq: 0.into(), // TODO: use actual channel seq
            })
            .into()
        }
        _ => return None,
    };

    Some(dispatch)
}
