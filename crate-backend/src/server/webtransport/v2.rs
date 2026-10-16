use super::*;
use common::v2::types::sync::dispatch::channel::{
    ChannelCreate, ChannelTyping, ChannelUpdate, DispatchChannel, MessageCreate,
};
use common::v2::types::sync::dispatch::global::{Ambient, DispatchGlobal};
use common::v2::types::sync::dispatch::room::RoomCreate;
use common::v2::types::sync::dispatch::{Dispatch, Ready};
use futures::SinkExt;
use kerosene_services::globals::messaging::Broadcast;
use kerosene_sync::util::Timeout;
use tokio::sync::watch;

use common::v1::types::presence::Presence;
use common::v1::types::{MemberListOp, MessageSync, Permission, SERVER_ROOM_ID, Session};
use common::v2::types::ConnectionId;
use common::v2::types::sync::stream::{channel, hello, member_list, room};
use common::v2::types::sync::transport::webtransport::Params;
use kerosene_services::services::member_lists::util::MemberListTarget as ServiceMemberListTarget;
use kerosene_sync::v2::codec::Codec;
use kerosene_sync::v2::transport::TransportStream;
use kerosene_sync::v2::transport::webtransport::WebtransportTransport;
use tokio::io::AsyncReadExt;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);

// rename to struct Connection?
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

pub async fn accept(globals: Globals, connection: Connection, params: SyncParams) -> Result<()> {
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

                    // send initial ambient state for the user
                    if let Some(user_id) = session.user_id() {
                        if let MessageSync::Ambient {
                            rooms,
                            roles,
                            channels,
                            threads,
                            room_members,
                            config,
                            ..
                        } = srv.cache.generate_ambient_message(user_id).await?
                        {
                            framed
                                .send(hello::Event::Dispatch(hello::Dispatch {
                                    inner: Box::new(
                                        DispatchGlobal::Ambient(Ambient {
                                            config,
                                            rooms,
                                            roles,
                                            channels,
                                            threads,
                                            room_members,
                                        })
                                        .into(),
                                    ),
                                    seq: 0,
                                }))
                                .await?;
                        }
                    }

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
                                    // TODO: use SyncErrorCode::Timeout
                                    return Err(Error::BadStatic("ping timeout"));
                                }
                            },
                            broadcast = broadcast_stream.next() => {
                                if let Some(msg) = broadcast {
                                    if let Some(msg) = broadcast_to_dispatch(state.globals.clone(), msg).await {
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
        StreamHeader::Room => {
            let session = state.session().await?;
            let codec = Codec::<room::Protocol>::new(state.params.clone());
            let (init, mut framed) = codec.accept(stream).await?;

            let srv = state.globals.services();
            srv.perms
                .for_room3(session.user_id(), init.room_id)
                .await?
                .ensure_view()?;

            // send typing states that were set before subscribing
            // PERF: this is terrible
            for (channel_id, user_id, until) in srv.channels.typing_list() {
                let Ok(channel) = srv.channels.get(channel_id, session.user_id()).await else {
                    continue;
                };
                if channel.room_id != Some(init.room_id) {
                    continue;
                }
                if !srv
                    .perms
                    .for_channel3(session.user_id(), channel_id)
                    .await
                    .map(|perms| perms.visible)
                    .unwrap_or(false)
                {
                    continue;
                }
                framed
                    .send(room::Event::Typing {
                        channel_id,
                        user_id,
                        until: until.into(),
                    })
                    .await?;
            }

            // we're tailing live events
            framed.send(room::Event::Subscribed).await?;

            // forward live typing events for channels in this room
            let mut broadcast_stream = state.globals.messaging().subscribe().await?;
            loop {
                tokio::select! {
                    broadcast = broadcast_stream.next() => {
                        let Some(Broadcast::Sync(sync)) = broadcast else {
                            continue;
                        };
                        let MessageSync::ChannelTyping {
                            channel_id,
                            user_id,
                            until,
                            room_id,
                        } = sync.message
                        else {
                            continue;
                        };
                        if room_id != Some(init.room_id) {
                            continue;
                        }
                        framed
                            .send(room::Event::Typing {
                                channel_id,
                                user_id,
                                until,
                            })
                            .await?;
                    }
                    _ = framed.next() => {
                        // room stream has no commands so this must be None (stream close)
                        break;
                    }
                }
            }

            Ok(())
        }
        StreamHeader::Channel => {
            let session = state.session().await?;
            let codec = Codec::<channel::Protocol>::new(state.params.clone());
            let (init, mut framed) = codec.accept(stream).await?;

            let srv = state.globals.services();
            srv.perms
                .for_channel3(session.user_id(), init.channel_id)
                .await?
                .ensure_view()?;

            // send initial snapshots for active flumes in this channel
            for entry in &srv.messages.flumes {
                let flume = entry.value();
                if flume.channel_id != init.channel_id {
                    continue;
                }
                let delta = srv.messages.flume_initial(flume).await?;
                framed
                    .send(channel::Event::FlumeInit {
                        channel_id: flume.channel_id,
                        message_id: *entry.key(),
                        delta,
                    })
                    .await?;
            }

            // we're tailing live events
            framed.send(channel::Event::Subscribed).await?;

            // forward live flume deltas for this channel
            let mut broadcast_stream = state.globals.messaging().subscribe().await?;
            loop {
                tokio::select! {
                    broadcast = broadcast_stream.next() => {
                        let Some(Broadcast::Sync(sync)) = broadcast else {
                            continue;
                        };
                        let MessageSync::FlumeDelta {
                            channel_id,
                            message_id,
                            delta,
                        } = sync.message
                        else {
                            continue;
                        };
                        if channel_id != init.channel_id {
                            continue;
                        }
                        framed
                            .send(channel::Event::FlumeDelta {
                                channel_id,
                                message_id,
                                delta,
                            })
                            .await?;
                    }
                    _ = framed.next() => {
                        // channel stream has no commands so this must be None (stream close)
                        break;
                    }
                }
            }

            Ok(())
        }
        StreamHeader::MemberList => {
            let session = state.session().await?;
            let codec = Codec::<member_list::Protocol>::new(state.params.clone());
            let (init, mut framed) = codec.accept(stream).await?;

            let srv = state.globals.services();
            let user_id = session.user_id();

            let target = match init.target {
                member_list::MemberListTarget::Room { room_id } => {
                    let mut perms = srv.perms.for_room3(user_id, room_id).await?.ensure_view()?;
                    if room_id == SERVER_ROOM_ID {
                        perms.needs(Permission::ServerOversee);
                    }
                    perms.check()?;
                    ServiceMemberListTarget::Room(room_id)
                }
                member_list::MemberListTarget::Channel { channel_id, .. } => {
                    srv.perms
                        .for_channel3(user_id, channel_id)
                        .await?
                        .ensure_view()?;
                    ServiceMemberListTarget::Channel(channel_id)
                }
            };

            let ranges = member_list_ranges(&init.ranges);

            let mut syncer = srv.member_lists.create_syncer(state.id);
            syncer.set_user_id(user_id).await;
            syncer.set_query(target.clone(), &ranges).await?;

            loop {
                tokio::select! {
                    msg = syncer.poll() => {
                        let msg = msg?;
                        if let Some(event) = member_list_event(msg) {
                            framed.send(event).await?;
                        }
                    }
                    command = framed.next() => {
                        match command.transpose()? {
                            Some(member_list::Command::Ranges { ranges }) => {
                                syncer
                                    .set_query(target.clone(), &member_list_ranges(&ranges))
                                    .await?;
                            }
                            None => break,
                        }
                    }
                }
            }

            Ok(())
        }
        _ => {
            let _session = state.session().await?;
            // TODO: do stuff with session
            Err(Error::Unimplemented)
        }
    }
}

/// convert member list ranges into service ranges
fn member_list_ranges(ranges: &[member_list::MemberListRange]) -> Vec<(u64, u64)> {
    ranges
        .iter()
        .map(|range| match range {
            member_list::MemberListRange::Static(start, end) => (*start, *end),
        })
        .collect()
}

/// convert a member list sync message into a member list stream event
fn member_list_event(msg: MessageSync) -> Option<member_list::Event> {
    let MessageSync::MemberListSync { ops, groups, .. } = msg else {
        return None;
    };

    let mut room_members = Vec::new();
    let mut thread_members = Vec::new();
    let mut users = Vec::new();
    let mut new_ops = Vec::with_capacity(ops.len());

    for op in ops {
        match op {
            MemberListOp::Sync {
                position,
                items,
                room_members: rm,
                thread_members: tm,
                users: us,
            } => {
                new_ops.push(member_list::Operation::Sync { position, items });
                room_members.extend(rm.into_iter().flatten());
                thread_members.extend(tm.into_iter().flatten());
                users.extend(us.into_iter().flatten());
            }
            MemberListOp::Insert {
                position,
                user_id,
                room_member,
                thread_member,
                user,
            } => {
                new_ops.push(member_list::Operation::Insert { position, user_id });
                room_members.extend(room_member);
                thread_members.extend(thread_member);
                users.extend(user.map(|user| *user));
            }
            MemberListOp::Delete { position, count } => {
                new_ops.push(member_list::Operation::Delete { position, count });
            }
        }
    }

    Some(member_list::Event {
        ops: new_ops,
        groups,
        room_members,
        thread_members,
        users,
    })
}

// TODO: implement this, slowly and painfully
async fn broadcast_to_dispatch(globals: Globals, broadcast: Broadcast) -> Option<Dispatch> {
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
        MessageSync::ChannelTyping {
            channel_id,
            user_id,
            until,
            room_id,
        } => {
            // typing in room channels requires a room subscription
            if room_id.is_some() {
                return None;
            }

            DispatchChannel::ChannelTyping(ChannelTyping {
                channel_id,
                user_id,
                until,
                room_id,
            })
            .into()
        }
        _ => return None,
    };

    Some(dispatch)
}
