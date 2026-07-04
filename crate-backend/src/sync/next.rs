//! work in progress redesign of the sync system

use crate::error::{Error, Result};
use crate::state::messaging::Broadcast;
use crate::sync::connection_queue::ConnectionQueue;
use crate::sync::permissions::AuthCheck;
use crate::sync::subscriptions::ConnectionSubscriptions;
use crate::sync::transport::{
    AnyTransport, Transport, TransportEvent, TransportSink, TransportStream, WebsocketTransport,
};
use crate::sync::util::{ConnectionState, HEARTBEAT_TIME, MAX_QUEUE_LEN, Timeout};
use crate::sync::{ConnectionErrorSeverity, severity};
use crate::{ServerState, prelude::*};
use common::v1::types::{
    ChannelId, DocumentBranchId, MessageClient, MessageEnvelope, MessagePayload, MessageSync,
    Permission, Session, SyncParams, SyncSubscribeDocument, SyncSubscribeMemberList,
    SyncSubscribeScript, SyncSubscription, UserId,
    document::DocumentUpdate,
    presence::Presence,
    voice::{VoiceStateUpdate, messages::SignallingCommand},
};
use common::v2::types::{ConnectionId, SessionId};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tracing::{Instrument, debug, error, trace, warn};

/// an authenticated connection
pub struct Connection2 {
    id: ConnectionId,
    session: Session,
    queue: ConnectionQueue,
    subscriptions: Box<ConnectionSubscriptions>,
    transport: Option<ConnectionTransport>,
    globals: Globals,
    rx: mpsc::Receiver<Command>,
}

pub struct ConnectionTransport {
    send: Box<dyn TransportSink>,
    recv: TransportStream,
    timeout: Timeout,
}

#[derive(Clone)]
pub struct ConnectionHandle {
    tx: mpsc::Sender<Command>,
    id: ConnectionId,
}

// TODO: rename to ConnectionCommand
/// a command for controlling a connection actor
pub enum Command {
    /// attach a transport to this connection and rewind to a seq
    Attach(Box<dyn Transport>, u64),

    /// shutdown this connection
    Shutdown,
}

// TODO: rename to ConnectionEvent
/// an event emitted by a connection actor
pub enum Event {
    /// this connection's transport was detached
    Detached,
}

// TODO: if something requires a user_id, send an error for guests but do not disconnect them
impl Connection2 {
    pub fn create(globals: Globals, session: Session) -> ConnectionHandle {
        let id = ConnectionId::new();
        let queue = ConnectionQueue::new(MAX_QUEUE_LEN);
        let subscriptions = Box::new(ConnectionSubscriptions::new(globals.clone(), id));
        let (tx, rx) = mpsc::channel(16);

        let me = Self {
            id,
            session,
            queue,
            subscriptions,
            transport: None,
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

    async fn spawn(mut self) {
        let mut sushi = self.globals.messaging().subscribe().await.unwrap();

        // init sync
        if let Err(err) = self.send_ready_state().await {
            error!("failed to init sync: {err}");
            return;
        }

        loop {
            // transport_futures event
            enum Tfe {
                Recv(Option<Result<TransportEvent>>),
                Timeout,
            }

            let transport_futures = async {
                if let Some(t) = &mut self.transport {
                    tokio::select! {
                        event = t.recv.next() => Tfe::Recv(event),
                        _ = tokio::time::sleep_until(t.timeout.get_instant()) => Tfe::Timeout,
                    }
                } else {
                    futures_util::future::pending().await
                }
            };

            tokio::select! {
                // poll transports
                event = transport_futures => {
                    match event {
                        Tfe::Recv(Some(Ok(event))) => {
                            if let Err(err) = self.handle_client(event).await {
                                error!("handle_client error: {err}");
                                // TODO: don't break on any error
                                break;
                            }
                        }
                        Tfe::Recv(Some(Err(_err))) => {
                            // TODO: handle Err
                        }
                        Tfe::Recv(None) => {
                            // TODO: handle None (transport closed)
                        }
                        Tfe::Timeout => {
                            if let Err(err) = self.handle_timeout().await {
                                error!("handle_timeout error: {err}");
                                break;
                            }
                            // TODO: handle Timeout::Close
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

            if let Some(t) = &mut self.transport {
                if let Err(err) = self.queue.drain(&mut *t.send, self.id).await {
                    error!("failed to drain messages: {err}");
                }
            }
        }
    }

    /// handle an event from the client
    async fn handle_client(&mut self, event: TransportEvent) -> Result<()> {
        match event {
            TransportEvent::Message(msg) => {
                match self.handle_message_client_inner(msg).await {
                    Ok(_) => {}
                    Err(err) => {
                        let t = self.transport.as_mut().ok_or_else(|| {
                            Error::BadStatic("transport lost during error handling")
                        })?;

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
                Ok(())
            }
            TransportEvent::Closed(clean) => self.handle_close(clean).await,
        }
    }

    /// handle a timeout
    async fn handle_timeout(&mut self) -> Result<()> {
        let Some(t) = &mut self.transport else {
            unreachable!("handle_timeout should never be called without a timeout")
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
                t.send.close().await?;
                // TODO: handle close, emit detach event
            }
        };
        Ok(())
    }

    async fn handle_close(&mut self, clean: bool) -> Result<()> {
        if clean {
            // set presence to offline
            if let Some(user_id) = self.session.user_id() {
                let srv = self.globals.services();
                if let Err(err) = srv.presence.set(user_id, Presence::offline()).await {
                    warn!("failed to set user {user_id} as offline: {err}");
                }
            }

            // clean up subscriptions
            // NOTE: does this clear document presence?
            if let Some(user_id) = self.session.user_id() {
                self.subscriptions.disconnect(user_id).await;
            }
        }

        // TODO: timer to invalidate connection after some amount of time

        self.transport = None;
        Ok(())
    }

    async fn handle_message_client_inner(&mut self, msg: MessageClient) -> Result<()> {
        let (_send, timeout) = {
            let t = self.transport.as_mut().ok_or_else(|| {
                Error::BadStatic("how did we receive a client event without an active transport?")
            })?;
            (&mut *t.send, &mut t.timeout)
        };

        trace!("{:#?}", msg);
        match msg {
            MessageClient::Hello { .. } => return Err(Error::BadStatic("already authenticated")),
            MessageClient::Presence { presence } => {
                let srv = self.globals.services();
                let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;
                let user = srv.users.get(user_id, None).await?;
                user.ensure_unsuspended()?;
                srv.presence.set(user_id, presence).await?;
            }
            // FIXME: allow guests to Pong
            MessageClient::Pong => {
                let srv = self.globals.services();
                let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;
                srv.presence.ping(user_id).await?;
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
                update,
            } => {
                self.handle_document_edit(channel_id, branch_id, update)
                    .await?
            }
            MessageClient::DocumentPresence {
                channel_id,
                branch_id,
                cursor_head,
                cursor_tail,
            } => {
                self.handle_document_presence(channel_id, branch_id, cursor_head, cursor_tail)
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
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        cursor_head: String,
        cursor_tail: Option<String>,
    ) -> Result<()> {
        let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;

        let srv = self.globals.services();
        let perms = srv.perms.for_channel(user_id, channel_id).await?;
        perms.ensure(Permission::ChannelView)?;

        if !self
            .subscriptions
            .is_document_subscribed(channel_id, branch_id)
        {
            return Err(Error::BadStatic("not subscribed to this document"));
        }

        srv.documents
            .broadcast_presence(
                (channel_id, branch_id),
                user_id,
                Some(self.id),
                cursor_head,
                cursor_tail,
            )
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
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        update: DocumentUpdate,
    ) -> Result<()> {
        let user_id = self.session.user_id().ok_or(Error::UnauthSession)?;
        let srv = self.globals.services();
        let perms = srv.perms.for_channel(user_id, channel_id).await?;
        perms.ensure(Permission::ChannelView)?;
        perms.ensure(Permission::DocumentEdit)?;

        if !self
            .subscriptions
            .is_document_subscribed(channel_id, branch_id)
        {
            return Err(Error::BadStatic("not subscribed to this document"));
        }

        srv.documents
            .apply_update((channel_id, branch_id), user_id, Some(self.id), &update.0)
            .await?;

        Ok(())
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
                        self.queue.push_sync(
                            MessageSync::ChannelTyping {
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
            Command::Attach(transport, seq) => {
                let (send, recv) = transport.split();
                self.transport = Some(ConnectionTransport {
                    send,
                    recv,
                    timeout: Timeout::for_ping(),
                });
                self.queue.rewind(seq)?;
            }
            Command::Shutdown => {
                if let Some(mut t) = self.transport.take() {
                    let _ = t.send.close().await;
                }

                // TODO: invalidate/remove this connection
            }
        }

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
                m => m,
            };
            self.queue.push_sync(msg, nonce);
        }

        Ok(())
    }
}

impl ConnectionHandle {
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    pub fn session_id(&self, session: &Session) -> SessionId {
        session.id
    }

    /// attach a transport to this connection and rewind
    pub fn attach(&self, transport: Box<dyn Transport>, seq: u64) {
        let _ = self.tx.try_send(Command::Attach(transport, seq));
    }

    /// shutdown this connection
    pub fn shutdown(&self) {
        let _ = self.tx.try_send(Command::Shutdown);
    }

    // /// stream events from this connection?
    // pub fn events(&self) { todo!() }
}

// TODO: later
// /// utility to accept new sync connections and do handshakes on them (wait for `Hello`)
// pub struct Handshake {
//     // ...
// }
//
// impl Handshake {
//     pub fn new(transport: AnyTransport) -> Self {
//         todo!()
//     }
//
//     pub async fn finish(self) -> Connection {
//         todo!()
//     }
// }
