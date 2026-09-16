use std::collections::HashMap;

use common::{
    v1::types::{
        ChannelId, MessageClient, MessageEnvelope, MessagePayload, MessageSync, Permission,
        RedexId, RoomId, Session, SyncSubscribeDocument, SyncSubscribeMemberList,
        SyncSubscribeScript, SyncSubscription,
        document::{DocumentStateVector, DocumentUpdate},
        voice::{VoiceStateUpdate, messages::SignallingCommand},
    },
    v2::types::{ConnectionId, SessionId},
};
use futures::FutureExt;
use kerosene_core::types::documents::EditContextId;
use kerosene_sync::{
    error::{ConnectionErrorSeverity, severity},
    permissions::AuthCheck,
    queue::ConnectionQueue,
    transport::{Transport, TransportEvent, TransportSink, TransportStream},
    util::{HEARTBEAT_TIME, MAX_QUEUE_LEN, Timeout},
};
use tokio::sync::mpsc;
use tracing::{Instrument, error, trace};

use crate::{
    globals::messaging::Broadcast, prelude::*,
    services::connections::subscriptions::ConnectionSubscriptions,
};

// TODO: impl Debug
// PERF: maybe don't copy globals to every connection?
// PERF: don't create a mpsc for connection, they can be expensive in terms of memory?
pub struct Connection {
    id: ConnectionId,
    session: Session,       // TODO: replace with minimal session object
    queue: ConnectionQueue, // TODO: remove
    subscriptions: Box<ConnectionSubscriptions>,
    transports: HashMap<ConnectionStream, ConnectionTransport>,
    globals: Globals,
    rx: mpsc::Receiver<Command>, // TODO: rename field?
}

pub struct ConnectionTransport {
    send: Box<dyn TransportSink>,
    recv: TransportStream,
    timeout: Timeout,
    queue: ConnectionQueue,
}

#[derive(Clone)]
pub struct ConnectionHandle {
    tx: mpsc::Sender<Command>,
    id: ConnectionId,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("id", &self.id)
            .field("session_id", &self.session.id)
            .field("user_id", &self.session.user_id())
            .field("transports", &self.transports)
            // TODO: subscriptions?
            .finish()
    }
}

impl std::fmt::Debug for ConnectionTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionTransport")
            .field("timeout", &self.timeout)
            .field("queue.len", &self.queue.len())
            .finish()
    }
}

impl std::fmt::Debug for ConnectionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionHandle")
            .field("id", &self.id)
            .finish()
    }
}

/// identifier for a webtransport stream
// TODO: identify streams via webtransport/quic stream id (add id fn to transport traits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ConnectionStream {
    Sync,
    Document(EditContextId),
    Voice(ChannelId),
    MemberList {
        room_id: Option<RoomId>,
        channel_id: Option<ChannelId>,
    },
    Script {
        channel_id: ChannelId,
        redex_id: RedexId,
    },
}

/// a command for controlling a connection actor
pub enum Command {
    /// attach a transport to this connection and rewind to a seq
    Attach {
        transport: Box<dyn Transport>,
        seq: u64,
        stream: AttachStream,
    },

    /// shutdown this connection
    Shutdown,
}

/// what stream to attach to
pub enum AttachStream {
    Sync,

    Document {
        context_id: EditContextId,
        state_vector: Option<DocumentStateVector>,
    },

    Voice {
        voice_state: VoiceStateUpdate,
        nonce: Option<String>,
    },

    MemberList {
        room_id: Option<RoomId>,
        channel_id: Option<ChannelId>,
        initial_ranges: Vec<(u64, u64)>,
    },

    Script {
        channel_id: ChannelId,
        script_id: RedexId,
    },
}

impl Connection {
    pub fn create(globals: Globals, session: Session) -> ConnectionHandle {
        let id = ConnectionId::new();
        let queue = ConnectionQueue::new(MAX_QUEUE_LEN);
        let subscriptions = Box::new(ConnectionSubscriptions::new(globals.clone(), id));
        let (tx, rx) = mpsc::channel(16);

        let mut me = Self {
            id,
            session,
            queue,
            subscriptions,
            transports: HashMap::new(),
            globals,
            rx,
        };

        let handle = ConnectionHandle { tx, id };

        tokio::spawn(
            async move {
                me.spawn().await;
            }
            .instrument(tracing::debug_span!("connection", id = %id)),
        );

        handle
    }

    // TODO: supervise connections in ServiceConnections, warn on unclean exit
    async fn spawn(&mut self) {
        let mut sushi = self.globals.messaging().subscribe().await.unwrap();

        // init sync
        if let Err(err) = self.send_ready_state().await {
            error!("failed to init sync: {err}");
            return;
        }

        loop {
            // transport_futures event
            enum Tfe {
                Recv(ConnectionStream, Option<Result<TransportEvent>>),
                Timeout(ConnectionStream),
            }

            let timeout_fut = match self.next_expiring_transport() {
                Some((stream, t)) => tokio::time::sleep_until(t.get_instant())
                    .map(move |_| stream)
                    .boxed(),
                None => futures_util::future::pending().boxed(),
            };

            let recv_fut = {
                let mut recvs = vec![];
                for (stream, t) in &mut self.transports {
                    recvs.push(t.recv.next().map(move |m| (*stream, m)).boxed());
                }

                if recvs.is_empty() {
                    futures_util::future::pending().boxed()
                } else {
                    futures_util::future::select_all(recvs).map(|f| f.0).boxed()
                }
            };

            let transport_futures = async move {
                tokio::select! {
                    (stream, event) = recv_fut => Tfe::Recv(stream, event),
                    stream = timeout_fut => Tfe::Timeout(stream),
                }
            };

            tokio::select! {
                // poll transports
                event = transport_futures => {
                    match event {
                        Tfe::Recv(stream, Some(Ok(event))) => {
                            if let Err(err) = self.handle_client(stream, event).await {
                                error!("handle_client error: {err}");
                                break;
                            }
                        }
                        Tfe::Recv(_stream, Some(Err(_err))) => {
                            // TODO: handle Err
                        }
                        Tfe::Recv(stream, None) => {
                            // unexpected disconnect, disconnect transport
                            self.handle_disconnect(stream, false).await;
                        }
                        Tfe::Timeout(stream) => {
                            if let Err(err) = self.handle_timeout(stream).await {
                                error!("handle_timeout error: {err}");
                                break;
                            }
                        }
                    }
                }

                // poll sushi
                Some(msg) = sushi.next() => {
                    if let Broadcast::Sync(sync) = msg {
                        if let Err(err) = self.queue_message(Box::new(sync.message), sync.nonce).await {
                            error!("failed to queue sushi message: {err}");
                        }
                    }
                }

                // poll subscriptions
                sub_res = self.subscriptions.poll() => {
                    match sub_res {
                        Ok(msg) => {
                            if let Err(err) = self.queue_message(Box::new(msg), None).await {
                                error!("failed to queue subscription message: {err}");
                            }
                        }
                        Err(err) => {
                            error!("subscription poll error: {err}");
                             // TODO: don't break on any error
                            break;
                        }
                    }
                }

                // handle commands
                Some(cmd) = self.rx.recv() => {
                    if let Err(err) = self.handle_command(cmd).await {
                        error!("handle_command error: {err}");
                        break;
                    }
                }
            }

            // drain queues
            for (stream, t) in self.transports.iter_mut() {
                if let Err(err) = t.queue.drain(&mut *t.send, self.id).await {
                    error!("failed to drain {:?} messages: {err}", stream);
                }
            }
        }
    }

    async fn send_ready_state(&mut self) -> Result<()> {
        let srv = self.globals.services();
        let user_id = self.session.user_id();

        let user = if let Some(uid) = user_id {
            let mut user = srv.users.get(uid, Some(uid)).await?;
            if !user.is_suspended() {
                user.presence = srv.presence.get(uid);
            }
            Some(user)
        } else {
            None
        };

        let application = if let Some(application_id) = self.session.app_id {
            let mut d = self.globals.begin_read().await?;
            Some(Box::new(d.application_get(application_id).await?))
        } else if let Some(uid) = user_id {
            let mut d = self.globals.begin_read().await?;
            d.application_get((*uid).into()).await.ok().map(Box::new)
        } else {
            None
        };

        let ready = MessagePayload::Ready {
            user: user.map(Box::new),
            application: application.clone(),
            session: self.session.clone(),
            conn: self.id,
            seq: 0,
        };

        self.queue.push(MessageEnvelope { payload: ready });

        if let Some(uid) = user_id {
            // Ambient
            let ambient = srv.cache.generate_ambient_message(uid).await?;
            self.queue.push_sync(ambient, None);

            // Typing
            let typing_states = srv.channels.typing_list();
            for (channel_id, typing_user_id, until) in typing_states {
                if let Ok(perms) = srv.perms.for_channel(uid, channel_id).await {
                    if perms.has(Permission::ChannelView) {
                        let channel = srv.channels.get(channel_id, None).await?;
                        self.queue.push_sync(
                            MessageSync::ChannelTyping {
                                room_id: channel.room_id,
                                channel_id,
                                user_id: typing_user_id,
                                until: until.into(),
                            },
                            None,
                        );
                    }
                }
            }

            // Voice
            let voice_states = srv.voice.state_list();
            for voice_state in voice_states {
                let vs = voice_state.inner();
                if let Ok(perms) = srv.perms.for_channel(uid, vs.channel_id).await {
                    let is_ours = self.session.user_id() == Some(vs.user_id);
                    if perms.has(Permission::ChannelView) || is_ours {
                        let mut vs = vs.to_owned();
                        if !is_ours {
                            vs.session_id = None;
                        }
                        self.queue.push_sync(
                            MessageSync::VoiceState {
                                user_id: vs.user_id,
                                state: Some(vs),
                                old_state: None,
                            },
                            None,
                        );
                    }
                }
            }

            // Flumes
            // NOTE: in the future, you will be required to subscribe to receive flumes
            for entry in &srv.messages.flumes {
                let flume = entry.value();
                if let Ok(perms) = srv.perms.for_channel3(Some(uid), flume.channel_id).await {
                    if perms.visible {
                        let delta = srv.messages.flume_initial(flume).await?;
                        self.queue.push_sync(
                            MessageSync::FlumeDelta {
                                channel_id: flume.channel_id,
                                message_id: *entry.key(),
                                delta,
                            },
                            None,
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Attach {
                transport,
                seq,
                stream,
            } => {
                // FIXME: send errors to transport rather than returning them
                let (send, recv) = transport.split();
                let transport = ConnectionTransport {
                    send,
                    recv,
                    timeout: Timeout::for_ping(),
                    queue: ConnectionQueue::new(MAX_QUEUE_LEN),
                };

                match stream {
                    AttachStream::Sync => {
                        self.transports.insert(ConnectionStream::Sync, transport);
                        self.queue.rewind(seq)?;
                        self.globals.services().connections.cancel_cleanup(self.id);
                    }
                    AttachStream::Document {
                        context_id,
                        state_vector,
                    } => {
                        let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;
                        self.subscriptions
                            .add_document_subscription(context_id, state_vector, user_id)
                            .await?;

                        self.transports
                            .insert(ConnectionStream::Document(context_id), transport);
                    }
                    AttachStream::Voice { voice_state, nonce } => {
                        let channel_id = voice_state.channel_id;
                        self.handle_voice_connect(voice_state, nonce).await?;
                        self.transports
                            .insert(ConnectionStream::Voice(channel_id), transport);
                    }
                    AttachStream::MemberList {
                        room_id,
                        channel_id,
                        initial_ranges,
                    } => {
                        let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;

                        let member_lists = vec![SyncSubscribeMemberList {
                            room_id,
                            channel_id,
                            ranges: initial_ranges,
                        }];

                        self.subscriptions
                            .set_subscription(
                                SyncSubscription {
                                    member_lists: Some(member_lists),
                                    documents: None,
                                    scripts: None,
                                },
                                user_id,
                            )
                            .await?;

                        self.transports.insert(
                            ConnectionStream::MemberList {
                                room_id,
                                channel_id,
                            },
                            transport,
                        );
                    }
                    AttachStream::Script {
                        channel_id,
                        script_id,
                    } => {
                        let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;

                        self.subscriptions
                            .set_subscription(
                                SyncSubscription {
                                    scripts: Some(vec![SyncSubscribeScript {
                                        channel_id,
                                        script_id,
                                    }]),
                                    documents: None,
                                    member_lists: None,
                                },
                                user_id,
                            )
                            .await?;

                        self.transports.insert(
                            ConnectionStream::Script {
                                channel_id,
                                redex_id: script_id,
                            },
                            transport,
                        );
                    }
                }
            }
            Command::Shutdown => {
                for (_, mut t) in self.transports.drain() {
                    let _ = t.send.close().await;
                }

                self.teardown().await;
            }
        }

        Ok(())
    }

    async fn handle_client(
        &mut self,
        stream: ConnectionStream,
        event: TransportEvent,
    ) -> Result<()> {
        match event {
            TransportEvent::Message(msg) => {
                // FIXME: handle document streams. currently this assumes all client messages are sent to the main sync stream.
                if let Err(err) = self.handle_message_client_inner(msg).await {
                    let t = self
                        .transports
                        .get_mut(&ConnectionStream::Sync)
                        .ok_or_else(|| Error::BadStatic("transport lost during error handling"))?;

                    let code = match &err {
                        Error::SyncError(c) => Some(c.clone()),
                        _ => None,
                    };
                    t.send
                        .send(MessageEnvelope {
                            payload: MessagePayload::Error {
                                error: err.to_string(),
                                code,
                            },
                        })
                        .await?;

                    let sev = severity(&err);
                    if matches!(
                        sev,
                        ConnectionErrorSeverity::Reconnect | ConnectionErrorSeverity::Fatal
                    ) {
                        t.send
                            .send(MessageEnvelope {
                                payload: MessagePayload::Reconnect {
                                    can_resume: sev == ConnectionErrorSeverity::Reconnect,
                                },
                            })
                            .await?;
                    }
                }
            }
            TransportEvent::Closed(clean) => self.handle_disconnect(stream, clean).await,
        }

        Ok(())
    }

    async fn handle_message_client_inner(&mut self, msg: MessageClient) -> Result<()> {
        let (_send, timeout) = {
            let t = self
                .transports
                .get_mut(&ConnectionStream::Sync)
                .ok_or_else(|| {
                    Error::BadStatic(
                        "how did we receive a client event without an active transport?",
                    )
                })?;
            (&mut *t.send, &mut t.timeout)
        };

        trace!("{:#?}", msg);

        // TODO: when using webtransport, respond with an error if any subscribe command is sent
        // TODO: when using webtransport, respond with an error if a document command is sent to a Sync stream or the wrong Document stream
        match msg {
            MessageClient::Hello(_) => return Err(Error::BadStatic("already authenticated")),
            MessageClient::Presence { presence } => {
                let srv = self.globals.services();
                let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;
                let user = srv.users.get(user_id, None).await?;
                user.ensure_unsuspended()?;
                srv.presence.set(self.id, user_id, presence);
            }
            MessageClient::Pong => {
                let srv = self.globals.services();
                if let Some(user_id) = self.session.user_id() {
                    srv.presence.ping(self.id, user_id);
                }
                *timeout = Timeout::Ping(tokio::time::Instant::now() + HEARTBEAT_TIME);
            }
            MessageClient::MemberListSubscribe {
                room_id,
                thread_id,
                ranges,
            } => {
                let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;

                let member_lists = if room_id.is_some() || thread_id.is_some() {
                    vec![SyncSubscribeMemberList {
                        room_id,
                        channel_id: thread_id,
                        ranges,
                    }]
                } else {
                    vec![]
                };

                self.subscriptions
                    .set_subscription(
                        SyncSubscription {
                            member_lists: Some(member_lists),
                            documents: None,
                            scripts: None,
                        },
                        user_id,
                    )
                    .await?;
            }
            MessageClient::VoiceConnect { voice_state, nonce } => {
                self.handle_voice_connect(voice_state, nonce).await?
            }
            MessageClient::VoiceDispatch {
                channel_id,
                nonce,
                command,
            } => {
                self.handle_voice_dispatch(channel_id, nonce, command)
                    .await?
            }
            MessageClient::DocumentSubscribe {
                channel_id,
                branch_id,
                state_vector,
            } => {
                let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;

                self.subscriptions
                    .set_subscription(
                        SyncSubscription {
                            documents: Some(vec![SyncSubscribeDocument {
                                channel_id,
                                branch_id,
                                redex_id: None,
                                state_vector,
                            }]),
                            member_lists: None,
                            scripts: None,
                        },
                        user_id,
                    )
                    .await?;
            }
            MessageClient::DocumentEdit {
                channel_id,
                branch_id,
                redex_id,
                update,
            } => {
                let context_id = if let Some(redex_id) = redex_id {
                    EditContextId::from_redex(channel_id, redex_id)
                } else {
                    EditContextId::from_prose(channel_id, branch_id)
                };
                self.handle_document_edit(context_id, update).await?
            }
            MessageClient::DocumentPresence {
                channel_id,
                branch_id,
                redex_id,
                cursor_head,
                cursor_tail,
            } => {
                let context_id = if let Some(redex_id) = redex_id {
                    EditContextId::from_redex(channel_id, redex_id)
                } else {
                    EditContextId::from_prose(channel_id, branch_id)
                };
                self.handle_document_presence(context_id, cursor_head, cursor_tail)
                    .await?
            }
            MessageClient::ScriptSubscribe {
                channel_id,
                script_id,
            } => {
                let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;

                self.subscriptions
                    .set_subscription(
                        SyncSubscription {
                            scripts: Some(vec![SyncSubscribeScript {
                                channel_id,
                                script_id,
                            }]),
                            documents: None,
                            member_lists: None,
                        },
                        user_id,
                    )
                    .await?;
            }
            MessageClient::Subscribe(subscribe) => self.handle_subscription(subscribe).await?,
        };

        Ok(())
    }

    async fn handle_document_presence(
        &mut self,
        context_id: EditContextId,
        cursor_head: String,
        cursor_tail: Option<String>,
    ) -> Result<()> {
        let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;

        let channel_id = context_id.channel_id();
        let branch_id = context_id.branch_id();

        let srv = self.globals.services();
        let perms = srv.perms.for_channel(user_id, channel_id).await?;
        perms.ensure(Permission::ChannelView)?;

        if !self.subscriptions.is_document_subscribed(context_id) {
            return Err(Error::BadStatic("not subscribed to this document"));
        }

        srv.documents
            .broadcast_presence(context_id, user_id, Some(self.id), cursor_head, cursor_tail)
            .await?;
        Ok(())
    }

    async fn handle_subscription(&mut self, subscribe: SyncSubscription) -> Result<()> {
        let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;

        self.subscriptions
            .set_subscription(subscribe, user_id)
            .await?;

        Ok(())
    }

    async fn handle_voice_connect(
        &mut self,
        vs: VoiceStateUpdate,
        nonce: Option<String>,
    ) -> Result<()> {
        let srv = self.globals.services();
        srv.voice
            .handle_voice_connect(self.session.clone(), self.id, vs, nonce)
            .await?;

        Ok(())
    }

    async fn handle_voice_dispatch(
        &mut self,
        channel_id: ChannelId,
        nonce: Option<String>,
        command: SignallingCommand,
    ) -> Result<()> {
        let srv = self.globals.services();
        srv.voice
            .handle_voice_dispatch(self.session.clone(), channel_id, nonce, command)
            .await?;

        Ok(())
    }

    async fn handle_document_edit(
        &mut self,
        context_id: EditContextId,
        update: DocumentUpdate,
    ) -> Result<()> {
        let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;
        let channel_id = context_id.channel_id();
        let branch_id = context_id.branch_id();

        let srv = self.globals.services();
        let perms = srv.perms.for_channel(user_id, channel_id).await?;
        perms.ensure(Permission::ChannelView)?;
        perms.ensure(Permission::DocumentEdit)?;

        if !self.subscriptions.is_document_subscribed(context_id) {
            return Err(Error::BadStatic("not subscribed to this document"));
        }

        srv.documents
            .apply_update(context_id, user_id, Some(self.id), &update.0)
            .await?;

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self), fields(id = %self.id))]
    pub async fn queue_message(
        &mut self,
        msg: Box<MessageSync>,
        nonce: Option<String>,
    ) -> Result<()> {
        let srv = self.globals.services();
        let auth_check = AuthCheck::for_message(&msg);
        let should_send = srv
            .perms
            .auth_check(&auth_check, &self.session, self.id)
            .await?;

        if should_send {
            let msg = match *msg {
                MessageSync::ChannelCreate { channel } => MessageSync::ChannelCreate {
                    channel: Box::new(srv.channels.get(channel.id, self.session.user_id()).await?),
                },
                MessageSync::ChannelUpdate { channel } => MessageSync::ChannelUpdate {
                    channel: Box::new(srv.channels.get(channel.id, self.session.user_id()).await?),
                },
                // FIXME: dont fetch from db for ephemeral messages
                MessageSync::MessageCreate { message } => MessageSync::MessageCreate {
                    message: srv
                        .messages
                        .get(message.channel_id, message.id, self.session.user_id())
                        .await?,
                },
                MessageSync::MessageUpdate { message } => MessageSync::MessageUpdate {
                    message: srv
                        .messages
                        .get(message.channel_id, message.id, self.session.user_id())
                        .await?,
                },
                MessageSync::VoiceState {
                    user_id,
                    mut state,
                    mut old_state,
                } => {
                    // strip session_id for voice states that aren't ours
                    let is_ours = self.session.user_id() == Some(user_id);
                    if !is_ours {
                        if let Some(s) = &mut state {
                            s.session_id = None;
                        }

                        if let Some(s) = &mut old_state {
                            s.session_id = None;
                        }
                    }

                    // if we don't have view perms in the new thread, treat it like a disconnect
                    if let Some(s) = &state {
                        let perms = srv.perms.for_channel(user_id, s.channel_id).await?;
                        if !perms.has(Permission::ChannelView) {
                            state = None;
                        }
                    }

                    MessageSync::VoiceState {
                        user_id,
                        state,
                        old_state,
                    }
                }
                MessageSync::AuditLogEntryCreate { mut entry } => {
                    entry.strip_request_metadata();
                    if self.session.user_id() != Some(entry.user_id) {
                        entry.strip_session();
                    }

                    MessageSync::AuditLogEntryCreate { entry }
                }
                m => m,
            };

            // route sync events
            // try to send document events to the matching document stream, falling back to the default sync stream
            // NOTE: do i want to drop document events if there is no document stream? if the client closed the stream, they probably don't want to receive events for that document anymore.
            let context_id = match &msg {
                MessageSync::DocumentEdit {
                    channel_id,
                    branch_id,
                    ..
                }
                | MessageSync::DocumentPresence {
                    channel_id,
                    branch_id,
                    ..
                }
                | MessageSync::DocumentSubscribed {
                    channel_id,
                    branch_id,
                    ..
                } => {
                    // FIXME: handle redex edit contexts
                    Some(EditContextId::from_prose(*channel_id, *branch_id))
                }
                _ => None,
            };

            if let Some(ctx) =
                context_id.and_then(|ctx| self.transports.get_mut(&ConnectionStream::Document(ctx)))
            {
                ctx.queue.push_sync(msg, nonce);
            } else if let Some(t) = self.transports.get_mut(&ConnectionStream::Sync) {
                t.queue.push_sync(msg, nonce);
            }
        }

        Ok(())
    }

    /// handle a timeout
    async fn handle_timeout(&mut self, stream: ConnectionStream) -> Result<()> {
        let t = self.transports.get_mut(&stream);
        let Some(t) = t else {
            // transport had to exist in order to timeout and cause handle_timeout to be called
            unreachable!("handle_timeout should never be called without a transport")
        };

        match &mut t.timeout {
            Timeout::Ping(_) => {
                let ping = MessageEnvelope {
                    payload: MessagePayload::Ping {},
                };
                t.send.send(ping).await?;
                // NOTE: do i need to drain anything? probably not
                // self.conn.drain(&mut *t.send).await?;
                t.timeout = Timeout::for_close();
            }
            Timeout::Close(_) => {
                let _ = t.send.close().await;

                match stream {
                    // if the main sync stream dies, shut down EVERYTHING
                    ConnectionStream::Sync => self.teardown().await,

                    // if a document times out, unsubscribe from it
                    ConnectionStream::Document(ctx_id) => {
                        self.transports.remove(&ConnectionStream::Document(ctx_id));
                        if let Some(user_id) = self.session.user_id() {
                            self.subscriptions
                                .remove_document_subscription(ctx_id, user_id)
                                .await;
                        }
                    }
                    ConnectionStream::Voice(channel_id) => {
                        self.transports.remove(&ConnectionStream::Voice(channel_id));
                        let _ = self
                            .handle_voice_dispatch(channel_id, None, SignallingCommand::Disconnect)
                            .await;
                    }
                    ConnectionStream::MemberList {
                        room_id,
                        channel_id,
                    } => {
                        self.transports.remove(&ConnectionStream::MemberList {
                            room_id,
                            channel_id,
                        });
                        self.subscriptions
                            .remove_member_list_subscription(room_id, channel_id);
                    }
                    ConnectionStream::Script {
                        channel_id,
                        redex_id,
                    } => {
                        self.transports.remove(&ConnectionStream::Script {
                            channel_id,
                            redex_id,
                        });
                        self.subscriptions
                            .remove_script_subscription(channel_id, redex_id);
                    }
                }
            }
        };

        Ok(())
    }

    /// this connection has been cleanly shutdown
    ///
    /// mark the user as offline, disconnect from subscriptions, etc
    async fn teardown(&mut self) {
        // remove this connection
        self.globals
            .services()
            .connections
            .connections
            .remove(&self.id);

        // set presence to offline
        if let Some(user_id) = self.session.user_id() {
            let srv = self.globals.services();
            srv.presence.disconnect(self.id, user_id);
        }

        // clean up subscriptions
        if let Some(user_id) = self.session.user_id() {
            self.subscriptions.disconnect(user_id).await;
        }

        // just to be safe
        self.transports.clear();
    }

    /// get the transport that will expire next
    fn next_expiring_transport(&self) -> Option<(ConnectionStream, Timeout)> {
        self.transports
            .iter()
            .map(|(stream, t)| (*stream, t.timeout))
            .min_by_key(|(_, t)| t.get_instant())
    }

    /// handle a transport being disconnected
    async fn handle_disconnect(&mut self, stream: ConnectionStream, clean: bool) {
        match stream {
            ConnectionStream::Sync => {
                if clean {
                    self.teardown().await
                } else {
                    self.transports.remove(&ConnectionStream::Sync);
                    self.globals
                        .services()
                        .connections
                        .schedule_cleanup(self.id);
                }
            }

            // TODO: check if below is correct
            ConnectionStream::Document(ctx_id) => {
                self.transports.remove(&ConnectionStream::Document(ctx_id));
                if let Some(user_id) = self.session.user_id() {
                    self.subscriptions
                        .remove_document_subscription(ctx_id, user_id)
                        .await;
                }
            }
            ConnectionStream::Voice(channel_id) => {
                self.transports.remove(&ConnectionStream::Voice(channel_id));
                let _ = self
                    .handle_voice_dispatch(channel_id, None, SignallingCommand::Disconnect)
                    .await;
            }
            ConnectionStream::MemberList {
                room_id,
                channel_id,
            } => {
                self.transports.remove(&ConnectionStream::MemberList {
                    room_id,
                    channel_id,
                });
                self.subscriptions
                    .remove_member_list_subscription(room_id, channel_id);
            }
            ConnectionStream::Script {
                channel_id,
                redex_id,
            } => {
                self.transports.remove(&ConnectionStream::Script {
                    channel_id,
                    redex_id,
                });
                self.subscriptions
                    .remove_script_subscription(channel_id, redex_id);
            }
        };
    }
}

impl ConnectionHandle {
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    /// attach a transport to this connection and rewind
    pub fn attach(&self, transport: Box<dyn Transport>, seq: u64) {
        self.attach_inner(transport, seq, AttachStream::Sync);
    }

    fn attach_inner(&self, transport: Box<dyn Transport>, seq: u64, stream: AttachStream) {
        let _ = self.tx.try_send(Command::Attach {
            transport,
            seq,
            stream,
        });
    }

    /// attach a dedicated document transport
    pub fn attach_document_transport(
        &self,
        context_id: EditContextId,
        transport: Box<dyn Transport>,
        sv: Option<DocumentStateVector>,
    ) {
        self.attach_inner(
            transport,
            0,
            AttachStream::Document {
                context_id,
                state_vector: sv,
            },
        );
    }

    /// attach a dedicated voice transport
    pub fn attach_voice(
        &self,
        transport: Box<dyn Transport>,
        voice_state: VoiceStateUpdate,
        nonce: Option<String>,
    ) {
        self.attach_inner(transport, 0, AttachStream::Voice { voice_state, nonce });
    }

    /// attach a dedicated member list transport
    pub fn attach_member_list(
        &self,
        transport: Box<dyn Transport>,
        room_id: Option<RoomId>,
        channel_id: Option<ChannelId>,
        initial_ranges: Vec<(u64, u64)>,
    ) {
        self.attach_inner(
            transport,
            0,
            AttachStream::MemberList {
                room_id,
                channel_id,
                initial_ranges,
            },
        );
    }

    /// attach a dedicated script transport
    pub fn attach_script(
        &self,
        transport: Box<dyn Transport>,
        channel_id: ChannelId,
        script_id: RedexId,
    ) {
        self.attach_inner(
            transport,
            0,
            AttachStream::Script {
                channel_id,
                script_id,
            },
        );
    }

    /// shutdown this connection
    pub fn shutdown(&self) {
        let _ = self.tx.try_send(Command::Shutdown);
    }
}
