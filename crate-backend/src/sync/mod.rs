use common::v1::types::{
    document::{DocumentStateVector, DocumentUpdate},
    sync::{SyncParams, SyncResume},
    voice::VoiceStateScreenshare,
    ChannelId, ConnectionId, SessionToken, UserId,
};
use std::time::Duration;
use std::{collections::VecDeque, sync::Arc};

use common::v1::types::emoji::EmojiOwner;
use common::v1::types::error::{ApiError, ErrorCode, SyncError};
use common::v1::types::presence::Presence;
use common::v1::types::util::Time;
use common::v1::types::voice::{SfuCommand, SfuPermissions, SignallingMessage, VoiceState};
use common::v1::types::{self, SERVER_ROOM_ID};
use common::v1::types::{
    DocumentBranchId, InviteTarget, InviteTargetId, MessageClient, MessageEnvelope, MessageSync,
    Permission, Session,
};
use tokio::time::Instant;
use tracing::{debug, error, trace, warn};

use crate::services::member_lists::{
    syncer::MemberListSyncer,
    util::{MemberListKey1, MemberListTarget},
};
use crate::sync::{permissions::AuthCheck, transport::TransportSink};
use crate::ServerState;
use crate::{
    error::{Error, Result},
    services::documents::DocumentSyncer,
};

pub mod permissions;
pub mod transport;

type WsMessage = axum::extract::ws::Message;

pub const HEARTBEAT_TIME: Duration = Duration::from_secs(30);
pub const CLOSE_TIME: Duration = Duration::from_secs(10);
const MAX_QUEUE_LEN: usize = 256;

pub enum Timeout {
    Ping(Instant),
    Close(Instant),
}

pub struct Connection {
    state: ConnectionState,
    s: Arc<ServerState>,
    queue: VecDeque<(Option<u64>, MessageEnvelope)>,
    seq_server: u64,
    seq_client: u64,
    id: ConnectionId,
    pub member_list: Box<ConnectionMemberListSyncer>,
    pub document: Box<DocumentSyncer>,
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Unauthed,
    Authenticated { session: Session },
    Disconnected { session: Session },
}

pub struct ConnectionMemberListSyncer {
    inner: MemberListSyncer,
    user_id: Option<UserId>,
    current_key: Option<MemberListKey1>,
    s: Arc<ServerState>,
}

// TODO(#996): remove this, merge with MemberListSyncer?
impl ConnectionMemberListSyncer {
    pub async fn set_user_id(&mut self, user_id: Option<UserId>) {
        self.user_id = user_id;
    }

    pub async fn set_query(
        &mut self,
        target: MemberListTarget,
        ranges: &[(u64, u64)],
    ) -> Result<()> {
        if let Some(key) = self.current_key.take() {
            let _ = self.inner.unsubscribe(key).await;
        }

        let srv = self.s.services();
        let key1 = match target {
            MemberListTarget::Room(room_id) => MemberListKey1::Room(room_id),
            MemberListTarget::Channel(channel_id) => {
                let channel = srv.channels.get(channel_id, None).await?;
                if let Some(room_id) = channel.room_id {
                    MemberListKey1::RoomChannel(room_id, channel_id)
                } else {
                    MemberListKey1::DmChannel(channel_id)
                }
            }
        };

        self.inner.subscribe(key1, ranges.to_vec()).await?;
        self.current_key = Some(key1);
        Ok(())
    }

    pub async fn clear_query(&mut self) {
        if let Some(key) = self.current_key.take() {
            let _ = self.inner.unsubscribe(key).await;
        }
    }

    pub async fn poll(&mut self) -> Result<MessageSync> {
        if let Some(user_id) = self.user_id {
            if let Some(msg) = self.inner.poll(user_id).await? {
                Ok(msg)
            } else {
                Err(Error::Internal(
                    "Member list poll returned None unexpectedly".to_string(),
                ))
            }
        } else {
            std::future::pending().await
        }
    }
}

impl Connection {
    pub fn new(s: Arc<ServerState>, _params: SyncParams) -> Self {
        let id = ConnectionId::new();

        let member_list = Box::new(ConnectionMemberListSyncer {
            inner: s.services().member_lists.create_syncer(id.into()),
            user_id: None,
            current_key: None,
            s: s.clone(),
        });

        Self {
            state: ConnectionState::Unauthed,
            queue: VecDeque::new(),
            seq_server: 0,
            seq_client: 0,
            id,
            member_list,
            document: Box::new(s.services().documents.create_syncer(id)),
            s,
        }
    }

    pub async fn disconnect(&mut self) {
        if let Some(session) = self.state.session() {
            if let Some(user_id) = session.user_id() {
                if let Err(err) = self.document.handle_disconnect(user_id).await {
                    error!("failed to clear document presence: {}", err);
                }
            }
        }

        // surely there's a way to do this with zero copies
        self.state = match &self.state {
            ConnectionState::Authenticated { session } => ConnectionState::Disconnected {
                session: session.clone(),
            },
            s => s.to_owned(),
        };
    }

    pub fn rewind(&mut self, seq: u64) -> Result<()> {
        let is_still_valid = self
            .queue
            .iter()
            .any(|(seq, _)| seq.is_some_and(|s| s <= self.seq_client));
        if is_still_valid {
            self.seq_client = seq;
            Ok(())
        } else {
            Err(SyncError::InvalidSeq.into())
        }
    }

    #[tracing::instrument(level = "debug", skip(self, transport, timeout), fields(id = self.get_id().to_string()))]
    pub async fn handle_message_client(
        &mut self,
        msg: MessageClient,
        transport: &mut dyn TransportSink,
        timeout: &mut Timeout,
    ) -> Result<()> {
        trace!("{:#?}", msg);
        match msg {
            MessageClient::Hello {
                token,
                resume: reconnect,
                presence,
            } => Box::pin(self.handle_hello(token, reconnect, presence, transport)).await?,
            MessageClient::Presence { presence } => {
                let session = match &self.state {
                    ConnectionState::Unauthed => return Err(SyncError::Unauthenticated.into()),
                    ConnectionState::Authenticated { session } => session,
                    ConnectionState::Disconnected { .. } => {
                        warn!("somehow recv msg while disconnected?");
                        return Ok(());
                    }
                };
                let srv = self.s.services();
                let user_id = session.user_id().ok_or(Error::UnauthSession)?;
                let user = srv.users.get(user_id, None).await?;
                user.ensure_unsuspended()?;
                srv.presence.set(user_id, presence).await?;
            }
            MessageClient::Pong => {
                let session = match &self.state {
                    ConnectionState::Unauthed => return Err(SyncError::Unauthenticated.into()),
                    ConnectionState::Authenticated { session } => session,
                    ConnectionState::Disconnected { .. } => {
                        panic!("somehow recv msg while disconnected?")
                    }
                };
                let srv = self.s.services();
                let user_id = session.user_id().ok_or(Error::UnauthSession)?;
                srv.presence.ping(user_id).await?;
                *timeout = Timeout::Ping(Instant::now() + HEARTBEAT_TIME);
            }
            MessageClient::MemberListSubscribe {
                room_id,
                thread_id,
                ranges,
            } => {
                let session = self
                    .state
                    .session()
                    .ok_or::<Error>(SyncError::Unauthenticated.into())?;
                let user_id = session.user_id().ok_or(Error::UnauthSession)?;
                let srv = self.s.services();

                // FIXME: validate that *exactly* one of room_id or thread_id is provided

                let target = if let Some(room_id) = room_id {
                    let perms = srv.perms.for_room(user_id, room_id).await?;
                    if room_id == SERVER_ROOM_ID {
                        perms.ensure(Permission::ServerOversee)?;
                    }
                    Some(MemberListTarget::Room(room_id))
                } else if let Some(thread_id) = thread_id {
                    let perms = srv.perms.for_channel(user_id, thread_id).await?;
                    perms.ensure(Permission::ChannelView)?;
                    Some(MemberListTarget::Channel(thread_id))
                } else {
                    None
                };

                if let Some(target) = target {
                    self.member_list.set_query(target, &ranges).await?;
                } else {
                    self.member_list.clear_query().await;
                }
            }
            MessageClient::VoiceDispatch { user_id, payload } => {
                Box::pin(self.handle_voice_dispatch(user_id, payload)).await?
            }
            MessageClient::DocumentSubscribe {
                channel_id,
                branch_id,
                state_vector,
            } => {
                Box::pin(self.handle_document_subscribe(channel_id, branch_id, state_vector))
                    .await?
            }
            MessageClient::DocumentEdit {
                channel_id,
                branch_id,
                update,
            } => Box::pin(self.handle_document_edit(channel_id, branch_id, update)).await?,
            MessageClient::DocumentPresence {
                channel_id,
                branch_id,
                cursor_head,
                cursor_tail,
            } => {
                Box::pin(self.handle_document_presence(
                    channel_id,
                    branch_id,
                    cursor_head,
                    cursor_tail,
                ))
                .await?
            }
        }
        Ok(())
    }

    async fn handle_document_presence(
        &mut self,
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        cursor_head: String,
        cursor_tail: Option<String>,
    ) -> Result<()> {
        let session = self
            .state
            .session()
            .ok_or::<Error>(SyncError::Unauthenticated.into())?;
        let user_id = session.user_id().ok_or(Error::UnauthSession)?;
        let srv = self.s.services();
        let perms = srv.perms.for_channel(user_id, channel_id).await?;
        perms.ensure(Permission::ChannelView)?;

        if !self.document.is_subscribed(&(channel_id, branch_id)) {
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

    async fn handle_hello(
        &mut self,
        token: SessionToken,
        reconnect: Option<SyncResume>,
        presence: Option<Presence>,
        transport: &mut dyn TransportSink,
    ) -> Result<()> {
        let srv = self.s.services();
        let session = srv
            .sessions
            .get_by_token(token)
            .await
            .map_err(|err| match err {
                Error::NotFound => SyncError::AuthFailure.into(),
                other => other,
            })?;

        // TODO: more forgiving reconnections?
        if let Some(r) = reconnect {
            debug!("attempting to resume");
            if let Some((_, mut conn)) = self.s.syncers.remove(&r.conn) {
                debug!("resume conn exists");
                if let Some(recon_session) = conn.state.session() {
                    debug!("resume session exists");
                    if session.id == recon_session.id {
                        debug!("session id matches, resuming");
                        conn.rewind(r.seq)?;
                        conn.push(
                            MessageEnvelope {
                                payload: types::MessagePayload::Resumed,
                            },
                            None,
                        );
                        std::mem::swap(self, &mut conn);
                        tracing::debug!("rehydrating syncer: {}", self.get_id());
                        return Ok(());
                    }
                }
            }
            return Err(Error::BadStatic("bad or expired reconnection info"));
        }

        if let ConnectionState::Authenticated { .. } = self.state {
            return Err(SyncError::AlreadyAuthenticated.into());
        }

        let user = if let Some(user_id) = session.user_id() {
            let srv = self.s.services();
            let mut user = srv.users.get(user_id, Some(user_id)).await?;
            if user.is_suspended() {
                Some(user)
            } else {
                let user_with_new_status = srv
                    .presence
                    .set(user_id, presence.unwrap_or(Presence::online()))
                    .await?;
                user.presence = user_with_new_status.presence;
                Some(user)
            }
        } else {
            None
        };

        let d = self.s.data();
        let application = if let Some(application_id) = session.app_id {
            Some(Box::new(d.application_get(application_id).await?))
        } else if let Some(uid) = session.user_id() {
            d.application_get((*uid).into()).await.ok().map(Box::new)
        } else {
            None
        };

        let msg = MessageEnvelope {
            payload: types::MessagePayload::Ready {
                user: user.map(Box::new),
                application,
                session: session.clone(),
                conn: self.get_id(),
                seq: 0,
            },
        };

        transport.send(msg).await?;

        self.seq_server += 1;

        if let Some(user_id) = session.user_id() {
            // send ambient data (rooms, channels, roles, etc.)
            let ambient = srv.cache.generate_ambient_message(user_id).await?;
            self.push_sync(ambient, None);

            // send typing states
            let typing_states = srv.channels.typing_list();
            for (channel_id, typing_user_id, until) in typing_states {
                if let Ok(perms) = srv.perms.for_channel(user_id, channel_id).await {
                    if perms.has(Permission::ChannelView) {
                        self.push_sync(
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

            // send voice states
            let voice_states = srv.voice.state_list();
            for voice_state in voice_states {
                if let Ok(perms) = srv.perms.for_channel(user_id, voice_state.channel_id).await {
                    let is_ours =
                        self.state.session().and_then(|s| s.user_id()) == Some(voice_state.user_id);
                    if perms.has(Permission::ChannelView) || is_ours {
                        let mut voice_state = voice_state.clone();
                        if !is_ours {
                            voice_state.session_id = None;
                        }
                        self.push_sync(
                            MessageSync::VoiceState {
                                user_id: voice_state.user_id,
                                state: Some(voice_state),
                                old_state: None,
                            },
                            None,
                        );
                    }
                }
            }
        }

        self.member_list.set_user_id(session.user_id()).await;
        self.document.set_user_id(session.user_id()).await;
        self.state = ConnectionState::Authenticated { session };
        Ok(())
    }

    async fn handle_voice_dispatch(
        &mut self,
        _user_id: UserId,
        payload: SignallingMessage,
    ) -> Result<()> {
        let Some(session) = self.state.session() else {
            return Err(Error::BadStatic("no session"));
        };
        let Some(user_id) = session.user_id() else {
            return Err(Error::BadStatic("no user"));
        };

        let srv = self.s.services();
        let user = srv.users.get(user_id, Some(user_id)).await?;
        user.ensure_unsuspended()?;

        match &payload {
            SignallingMessage::VoiceState { state: Some(state) } => {
                let perms = srv.perms.for_channel(user_id, state.channel_id).await?;
                perms.ensure(Permission::ChannelView)?;
                let thread = srv.channels.get(state.channel_id, Some(user_id)).await?;
                thread.ensure_unarchived()?;
                thread.ensure_unremoved()?;
                perms.ensure_unlocked()?;
                let old_state = srv.voice.state_get(user_id);
                let mut state = VoiceState {
                    user_id,
                    channel_id: state.channel_id,
                    session_id: Some(session.id),
                    connection_id: Some(self.id),
                    joined_at: Time::now_utc(),
                    mute: false,
                    deaf: false,
                    self_deaf: state.self_deaf,
                    self_mute: state.self_mute,
                    self_video: state.self_video,
                    screenshare: match (old_state, state.screenshare.as_ref()) {
                        (Some(old), Some(new)) => Some(VoiceStateScreenshare {
                            started_at: old
                                .screenshare
                                .map(|s| s.started_at)
                                .unwrap_or_else(|| Time::now_utc()),
                            thumbnail: new.thumbnail,
                        }),
                        (None, Some(new)) => Some(VoiceStateScreenshare {
                            started_at: Time::now_utc(),
                            thumbnail: new.thumbnail,
                        }),
                        (_, None) => None,
                    },
                    // TODO: suppress by default in broadcast room
                    suppress: false,
                    requested_to_speak_at: None,
                };
                if let Some(room_id) = thread.room_id {
                    let rm = self.s.data().room_member_get(room_id, user_id).await?;
                    state.mute = rm.mute;
                    state.deaf = rm.deaf;
                }
                srv.voice.alloc_sfu(state.channel_id).await?;
                if let Err(err) = self.s.broadcast_sfu(SfuCommand::VoiceState {
                    user_id,
                    state: Some(state),
                    permissions: SfuPermissions {
                        speak: perms.has(Permission::VoiceSpeak),
                        video: perms.has(Permission::VoiceVideo),
                        priority: perms.has(Permission::VoicePriority),
                    },
                }) {
                    error!("failed to send to sushi_sfu: {err}");
                }
                return Ok(());
            }
            SignallingMessage::VoiceState { state: None } => {
                if let Err(err) = self.s.broadcast_sfu(SfuCommand::VoiceState {
                    user_id,
                    state: None,
                    permissions: SfuPermissions {
                        speak: false,
                        video: false,
                        priority: false,
                    },
                }) {
                    error!("failed to send to sushi_sfu: {err}");
                }
                return Ok(());
            }
            SignallingMessage::Offer { .. } => {
                // TODO: also verify sdp and/or send permissions to sfu instead of only parsing tracks
                // let perms = srv.perms.for_thread(user_id, voice_state.thread_id).await?;
                // if tracks.iter().any(|t| t.kind == MediaKindSerde::Audio) {
                //     perms.ensure(Permission::VoiceSpeak)?;
                // }
                // if tracks.iter().any(|t| t.kind == MediaKindSerde::Video) {
                //     perms.ensure(Permission::VoiceVideo)?;
                // }
            }
            _ => {}
        }

        if let Err(err) = self.s.broadcast_sfu(SfuCommand::Signalling {
            user_id,
            inner: payload,
        }) {
            error!("failed to send to sushi_sfu: {err}");
        }
        Ok(())
    }

    async fn handle_document_subscribe(
        &mut self,
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        state_vector: Option<DocumentStateVector>,
    ) -> Result<()> {
        let session = self
            .state
            .session()
            .ok_or::<Error>(SyncError::Unauthenticated.into())?;
        let user_id = session.user_id().ok_or(Error::UnauthSession)?;
        let srv = self.s.services();
        let perms = srv.perms.for_channel(user_id, channel_id).await?;
        perms.ensure(Permission::ChannelView)?;

        let branch = self
            .s
            .data()
            .document_branch_get(channel_id, branch_id)
            .await;
        match branch {
            Ok(branch) => {
                if branch.private && branch.creator_id != user_id {
                    return Err(Error::ApiError(ApiError::from_code(
                        ErrorCode::UnknownDocumentBranch,
                    )));
                }
            }
            Err(_) if *branch_id == *channel_id => {
                // this is the default branch
            }
            Err(_) => {
                return Err(Error::ApiError(ApiError::from_code(
                    ErrorCode::UnknownDocumentBranch,
                )));
            }
        }

        self.document
            .set_context_id((channel_id, branch_id), state_vector)
            .await?;

        Ok(())
    }

    async fn handle_document_edit(
        &mut self,
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        update: DocumentUpdate,
    ) -> Result<()> {
        let session = self
            .state
            .session()
            .ok_or::<Error>(SyncError::Unauthenticated.into())?;
        let user_id = session.user_id().ok_or(Error::UnauthSession)?;
        let srv = self.s.services();
        let perms = srv.perms.for_channel(user_id, channel_id).await?;
        perms.ensure(Permission::ChannelView)?;
        perms.ensure(Permission::DocumentEdit)?;

        if !self.document.is_subscribed(&(channel_id, branch_id)) {
            return Err(Error::BadStatic("not subscribed to this document"));
        }

        srv.documents
            .apply_update((channel_id, branch_id), user_id, Some(self.id), &update.0)
            .await?;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self), fields(id = self.get_id().to_string()))]
    pub async fn queue_message(
        &mut self,
        msg: Box<MessageSync>,
        nonce: Option<String>,
    ) -> Result<()> {
        let mut session = match &self.state {
            ConnectionState::Authenticated { session }
            | ConnectionState::Disconnected { session } => session.clone(),
            _ => return Ok(()),
        };

        match &self.state {
            ConnectionState::Disconnected { .. }
                if self.seq_server > self.seq_client + MAX_QUEUE_LEN as u64 =>
            {
                self.s.syncers.remove(&self.id);
                return Err(Error::BadStatic("expired session"));
            }
            _ => {}
        }

        let srv = self.s.services();
        let auth_check = AuthCheck::for_message(&msg);
        let should_send = srv.perms.auth_check(&auth_check, &session, self.id).await?;

        if should_send {
            let srv = self.s.services();
            let msg = match *msg {
                MessageSync::ChannelCreate { channel } => MessageSync::ChannelCreate {
                    channel: Box::new(srv.channels.get(channel.id, session.user_id()).await?),
                },
                MessageSync::ChannelUpdate { channel } => MessageSync::ChannelUpdate {
                    channel: Box::new(srv.channels.get(channel.id, session.user_id()).await?),
                },
                MessageSync::MessageCreate { message } => MessageSync::MessageCreate {
                    message: srv
                        .messages
                        .get(message.channel_id, message.id, session.user_id().unwrap())
                        .await?,
                },
                MessageSync::MessageUpdate { message } => MessageSync::MessageUpdate {
                    message: srv
                        .messages
                        .get(message.channel_id, message.id, session.user_id().unwrap())
                        .await?,
                },
                MessageSync::VoiceState {
                    user_id,
                    mut state,
                    mut old_state,
                } => {
                    // strip session_id for voice states that aren't ours
                    let is_ours = self.state.session().and_then(|s| s.user_id()) == Some(user_id);
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
                        let perms = self
                            .s
                            .services()
                            .perms
                            .for_channel(user_id, s.channel_id)
                            .await?;
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
            self.push_sync(msg, nonce);
        }

        Ok(())
    }

    fn push_sync(&mut self, sync: MessageSync, nonce: Option<String>) {
        let seq = self.seq_server;
        let msg = MessageEnvelope {
            payload: types::MessagePayload::Sync {
                data: Box::new(sync),
                seq,
                nonce,
            },
        };
        self.push(msg, Some(seq));
        self.seq_server += 1;
    }

    fn push(&mut self, msg: MessageEnvelope, seq: Option<u64>) {
        self.queue.push_front((seq, msg));
        self.queue.truncate(MAX_QUEUE_LEN);
    }

    #[tracing::instrument(level = "debug", skip(self, transport), fields(id = self.get_id().to_string()))]
    pub async fn drain(&mut self, transport: &mut dyn TransportSink) -> Result<()> {
        let last_seen = self.seq_client;
        let mut high_water_mark = last_seen;

        let queue = &self.queue;

        for (seq, msg) in queue.iter().rev() {
            if seq.is_none_or(|s| s > last_seen) {
                transport.send(msg.clone()).await?;

                if let Some(seq) = *seq {
                    high_water_mark = high_water_mark.max(seq);
                }
            }
        }
        self.seq_client = high_water_mark;
        self.queue.retain(|(seq, _)| seq.is_some());
        Ok(())
    }

    pub fn get_id(&self) -> ConnectionId {
        self.id
    }

    pub fn state(&self) -> &ConnectionState {
        &self.state
    }
}

impl ConnectionState {
    pub fn session(&self) -> Option<&Session> {
        match self {
            ConnectionState::Unauthed => None,
            ConnectionState::Authenticated { session } => Some(session),
            ConnectionState::Disconnected { session } => Some(session),
        }
    }
}

impl Timeout {
    pub fn for_ping() -> Self {
        Timeout::Ping(Instant::now() + HEARTBEAT_TIME)
    }

    pub fn for_close() -> Self {
        Timeout::Close(Instant::now() + CLOSE_TIME)
    }

    pub fn get_instant(&self) -> Instant {
        match self {
            Timeout::Ping(instant) => *instant,
            Timeout::Close(instant) => *instant,
        }
    }
}
