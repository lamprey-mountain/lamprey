use common::{
    v1::types::{
        ChannelId, DocumentBranchId, MessageClient, MessageEnvelope, MessagePayload, MessageSync,
        Permission, Session, SyncSubscribeDocument, SyncSubscribeMemberList, SyncSubscribeScript,
        SyncSubscription,
        document::DocumentUpdate,
        presence::Presence,
        voice::{VoiceStateUpdate, messages::SignallingCommand},
    },
    v2::types::{ConnectionId, SessionId},
};
use kerosene_sync::{
    permissions::AuthCheck,
    queue::ConnectionQueue,
    transport::{Transport, TransportEvent, TransportSink, TransportStream},
    util::{HEARTBEAT_TIME, MAX_QUEUE_LEN, Timeout},
};
use tokio::sync::mpsc;
use tracing::{Instrument, error, trace, warn};

use crate::{prelude::*, services::connections::subscriptions::ConnectionSubscriptions};

// TODO: impl Debug
pub struct Connection {
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

// TODO: impl Debug
#[derive(Clone)]
pub struct ConnectionHandle {
    tx: mpsc::Sender<Command>,
    id: ConnectionId,
}

/// a command for controlling a connection actor
pub enum Command {
    /// attach a transport to this connection and rewind to a seq
    Attach(Box<dyn Transport>, u64),

    /// shutdown this connection
    Shutdown,
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

    async fn spawn(&mut self) {
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

                // poll subscriptions
                sub_res = self.subscriptions.poll() => {
                    match sub_res {
                        Ok(msg) => {
                            // TODO: Need queue_message implementation
                            // if let Err(err) = self.queue_message(Box::new(msg), None).await {
                            //     error!("failed to queue subscription message: {err}");
                            // }
                            self.queue.push_sync(msg, None);
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

    async fn handle_client(&mut self, event: TransportEvent) -> Result<()> {
        match event {
            TransportEvent::Message(msg) => {
                match self.handle_message_client_inner(msg).await {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Error handling message: {:?}", err);
                    }
                }
                Ok(())
            }
            TransportEvent::Closed(clean) => self.handle_close(clean).await,
        }
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
            MessageClient::Pong => {
                let srv = self.globals.services();
                if let Some(user_id) = self.session.user_id() {
                    srv.presence.ping(user_id).await?;
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
}
