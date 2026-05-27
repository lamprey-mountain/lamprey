use common::v1::types::{
    document::DocumentUpdate,
    sync::{SyncParams, SyncResume},
    voice::{messages::SfuCommand, VoiceStateUpdate},
    ChannelId, ConnectionId, SessionToken, SyncSubscribeDocument, SyncSubscribeMemberList,
    SyncSubscribeScript, SyncSubscription,
};
use std::sync::Arc;

use common::v1::types;
use common::v1::types::error::SyncErrorCode;
use common::v1::types::presence::Presence;
use common::v1::types::voice::messages::SignallingCommand;
use common::v1::types::{
    DocumentBranchId, MessageClient, MessageEnvelope, MessageSync, Permission, UserId,
};
use tokio::time::Instant;
use tracing::{debug, trace, warn};

pub mod connection_queue;
pub mod permissions;
pub mod subscriptions;
pub mod transport;
pub mod util;

use crate::error::{Error, Result};
use crate::sync::util::{ConnectionState, Timeout, HEARTBEAT_TIME, MAX_QUEUE_LEN};
use crate::sync::{
    connection_queue::ConnectionQueue, permissions::AuthCheck,
    subscriptions::ConnectionSubscriptions, transport::TransportSink,
};
use crate::ServerState;

type WsMessage = axum::extract::ws::Message;

pub struct Connection {
    state: ConnectionState,
    s: Arc<ServerState>,
    queue: ConnectionQueue,
    id: ConnectionId,
    pub subscriptions: Box<ConnectionSubscriptions>,
}

impl Connection {
    pub fn new(s: Arc<ServerState>, _params: SyncParams) -> Self {
        let id = ConnectionId::new();

        Self {
            state: ConnectionState::Unauthed,
            queue: ConnectionQueue::new(MAX_QUEUE_LEN),
            id,
            subscriptions: Box::new(ConnectionSubscriptions::new(s.clone(), id)),
            s,
        }
    }

    pub async fn disconnect(&mut self) {
        if let Some(session) = self.state.session() {
            if let Some(user_id) = session.user_id() {
                self.subscriptions.disconnect(user_id).await;
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
        self.queue.rewind(seq)
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
                    ConnectionState::Unauthed => return Err(SyncErrorCode::Unauthenticated.into()),
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
                    ConnectionState::Unauthed => return Err(SyncErrorCode::Unauthenticated.into()),
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
                    .ok_or::<Error>(SyncErrorCode::Unauthenticated.into())?;
                let user_id = session.user_id().ok_or(Error::UnauthSession)?;

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
                Box::pin(self.handle_voice_connect(voice_state, nonce)).await?
            }
            MessageClient::VoiceDispatch {
                channel_id,
                nonce,
                command,
            } => Box::pin(self.handle_voice_dispatch(channel_id, nonce, command)).await?,
            MessageClient::DocumentSubscribe {
                channel_id,
                branch_id,
                state_vector,
            } => {
                let session = self
                    .state
                    .session()
                    .ok_or::<Error>(SyncErrorCode::Unauthenticated.into())?;
                let user_id = session.user_id().ok_or(Error::UnauthSession)?;

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
            MessageClient::ScriptSubscribe {
                channel_id,
                script_id,
            } => {
                let session = self
                    .state
                    .session()
                    .ok_or::<Error>(SyncErrorCode::Unauthenticated.into())?;
                let user_id = session.user_id().ok_or(Error::UnauthSession)?;

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
            MessageClient::Subscribe(subscribe) => {
                Box::pin(self.handle_subscription(subscribe)).await?
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
            .ok_or::<Error>(SyncErrorCode::Unauthenticated.into())?;
        let user_id = session.user_id().ok_or(Error::UnauthSession)?;

        let srv = self.s.services();
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
        let session = self
            .state
            .session()
            .ok_or::<Error>(SyncErrorCode::Unauthenticated.into())?;
        let user_id = session.user_id().ok_or(Error::UnauthSession)?;

        self.subscriptions
            .set_subscription(subscribe, user_id)
            .await?;

        Ok(())
    }

    async fn handle_hello(
        &mut self,
        token: SessionToken,
        reconnect: Option<SyncResume>,
        presence: Option<Presence>,
        _transport: &mut dyn TransportSink,
    ) -> Result<()> {
        let srv = self.s.services();
        let session = srv
            .sessions
            .get_by_token(token)
            .await
            .map_err(|err| match err {
                Error::NotFound => SyncErrorCode::AuthFailure.into(),
                other => other,
            })?;

        // TODO: more forgiving reconnections?
        if let Some(r) = reconnect {
            debug!("attempting to resume");
            if let Some((_, mut conn)) = self.s.services.connections.live.remove(&r.conn) {
                debug!("resume conn exists");
                if let Some(recon_session) = conn.state.session() {
                    debug!("resume session exists");
                    if session.id == recon_session.id {
                        debug!("session id matches, resuming");
                        conn.rewind(r.seq)?;
                        conn.queue.push(MessageEnvelope {
                            payload: types::MessagePayload::Resumed,
                        });
                        std::mem::swap(self, &mut conn);
                        tracing::debug!("rehydrating syncer: {}", self.get_id());
                        return Ok(());
                    }
                }
            }
            return Err(Error::BadStatic("bad or expired reconnection info"));
        }

        if let ConnectionState::Authenticated { .. } = self.state {
            return Err(SyncErrorCode::AlreadyAuthenticated.into());
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

        let application = if let Some(application_id) = session.app_id {
            let mut d = self.s.data();
            Some(Box::new(d.application_get(application_id).await?))
        } else if let Some(uid) = session.user_id() {
            let mut d = self.s.data();
            d.application_get((*uid).into()).await.ok().map(Box::new)
        } else {
            None
        };

        let ready = types::MessagePayload::Ready {
            user: user.map(Box::new),
            application: application.clone(),
            session: session.clone(),
            conn: self.get_id(),
            seq: 0,
        };

        debug!("send ready {ready:?}");

        self.queue.push(types::MessageEnvelope { payload: ready });

        if let Some(user_id) = session.user_id() {
            // send ambient data (rooms, channels, roles, etc.)
            let ambient = srv.cache.generate_ambient_message(user_id).await?;
            debug!("send ambient");
            self.queue.push_sync(ambient, None);

            // send typing states
            let typing_states = srv.channels.typing_list();
            for (channel_id, typing_user_id, until) in typing_states {
                if let Ok(perms) = srv.perms.for_channel(user_id, channel_id).await {
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

            // send voice states
            let voice_states = srv.voice.state_list();
            for voice_state in voice_states {
                let vs = voice_state.inner();
                if let Ok(perms) = srv.perms.for_channel(user_id, vs.channel_id).await {
                    let is_ours =
                        self.state.session().and_then(|s| s.user_id()) == Some(vs.user_id);
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

            // send flumes
            for entry in &srv.messages.flumes {
                let flume = entry.value();
                if let Ok(perms) = srv
                    .perms
                    .for_channel3(Some(user_id), flume.channel_id)
                    .await
                {
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

            // TODO: send document presence
        }

        self.state = ConnectionState::Authenticated { session };
        Ok(())
    }

    async fn handle_voice_connect(
        &mut self,
        vs: VoiceStateUpdate,
        nonce: Option<String>,
    ) -> Result<()> {
        let session = self
            .state
            .session()
            .ok_or::<Error>(SyncErrorCode::Unauthenticated.into())?;
        let user_id = session.user_id().ok_or(Error::UnauthSession)?;

        let srv = self.s.services();
        srv.voice.state_create(user_id, vs).await?;

        Ok(())
    }

    async fn handle_voice_dispatch(
        &mut self,
        channel_id: ChannelId,
        nonce: Option<String>,
        command: SignallingCommand,
    ) -> Result<()> {
        let srv = self.s.services();
        let user_id = self.user_id().unwrap();
        if let Some(sfu) = srv.voice.sfu_by_channel(
            srv.voice
                .state_get(channel_id, user_id)
                .map(|s| s.inner().channel_id)
                .ok_or(Error::BadStatic("state not found"))?,
        ) {
            sfu.send(SfuCommand::Signalling {
                user_id,
                channel_id,
                inner: command,
            });
        }
        Ok(())
    }

    fn user_id(&self) -> Option<UserId> {
        todo!()
    }

    // async fn handle_voice_dispatch_old(
    //     &mut self,
    //     _user_id: UserId,
    //     payload: SignallingMessage,
    // ) -> Result<()> {
    //     let Some(session) = self.state.session() else {
    //         return Err(Error::BadStatic("no session"));
    //     };
    //     let Some(user_id) = session.user_id() else {
    //         return Err(Error::BadStatic("no user"));
    //     };

    //     let srv = self.s.services();
    //     let user = srv.users.get(user_id, Some(user_id)).await?;
    //     user.ensure_unsuspended()?;

    //     match &payload {
    //         SignallingMessage::VoiceState { state: Some(state) } => {
    //             let perms = srv.perms.for_channel(user_id, state.channel_id).await?;
    //             perms.ensure(Permission::ChannelView)?;
    //             let thread = srv.channels.get(state.channel_id, Some(user_id)).await?;
    //             thread.ensure_unarchived()?;
    //             thread.ensure_unremoved()?;
    //             perms.ensure_unlocked()?;
    //             let old_state = srv.voice.state_get(user_id);
    //             let mut state = VoiceState {
    //                 user_id,
    //                 channel_id: state.channel_id,
    //                 session_id: Some(session.id),
    //                 connection_id: Some(self.id),
    //                 joined_at: Time::now_utc(),
    //                 mute: false,
    //                 deaf: false,
    //                 self_deaf: state.self_deaf,
    //                 self_mute: state.self_mute,
    //                 self_video: state.self_video,
    //                 screenshare: match (old_state, state.screenshare.as_ref()) {
    //                     (Some(old), Some(new)) => Some(VoiceStateScreenshare {
    //                         started_at: old
    //                             .screenshare
    //                             .map(|s| s.started_at)
    //                             .unwrap_or_else(|| Time::now_utc()),
    //                         thumbnail: new.thumbnail,
    //                     }),
    //                     (None, Some(new)) => Some(VoiceStateScreenshare {
    //                         started_at: Time::now_utc(),
    //                         thumbnail: new.thumbnail,
    //                     }),
    //                     (_, None) => None,
    //                 },
    //                 // TODO: suppress by default in broadcast room
    //                 suppress: false,
    //                 requested_to_speak_at: None,
    //             };
    //             if let Some(room_id) = thread.room_id {
    //                 let rm = self.s.data().room_member_get(room_id, user_id).await?;
    //                 state.mute = rm.mute;
    //                 state.deaf = rm.deaf;
    //             }
    //             srv.voice.alloc_sfu(state.channel_id).await?;
    //             if let Err(err) = self.s.broadcast_sfu(SfuCommand::VoiceState {
    //                 user_id,
    //                 state: Some(state),
    //                 permissions: SfuPermissions(
    //                     (perms.has(Permission::VoiceSpeak) as u8) << 0
    //                         | (perms.has(Permission::VoiceVideo) as u8) << 1
    //                         | (perms.has(Permission::VoicePriority) as u8) << 2,
    //                 ),
    //             }) {
    //                 error!("failed to send to sushi_sfu: {err}");
    //             }
    //             return Ok(());
    //         }
    //         SignallingMessage::VoiceState { state: None } => {
    //             if let Err(err) = self.s.broadcast_sfu(SfuCommand::VoiceState {
    //                 user_id,
    //                 state: None,
    //                 permissions: SfuPermissions(0),
    //             }) {
    //                 error!("failed to send to sushi_sfu: {err}");
    //             }
    //             return Ok(());
    //         }
    //         SignallingMessage::Offer { .. } => {
    //             // TODO: also verify sdp and/or send permissions to sfu instead of only parsing tracks
    //             // let perms = srv.perms.for_thread(user_id, voice_state.thread_id).await?;
    //             // if tracks.iter().any(|t| t.kind == MediaKindSerde::Audio) {
    //             //     perms.ensure(Permission::VoiceSpeak)?;
    //             // }
    //             // if tracks.iter().any(|t| t.kind == MediaKindSerde::Video) {
    //             //     perms.ensure(Permission::VoiceVideo)?;
    //             // }
    //         }
    //         _ => {}
    //     }

    //     if let Err(err) = self.s.broadcast_sfu(SfuCommand::Signalling {
    //         user_id,
    //         inner: payload,
    //     }) {
    //         error!("failed to send to sushi_sfu: {err}");
    //     }
    //     Ok(())
    // }

    async fn handle_document_edit(
        &mut self,
        channel_id: ChannelId,
        branch_id: DocumentBranchId,
        update: DocumentUpdate,
    ) -> Result<()> {
        let session = self
            .state
            .session()
            .ok_or::<Error>(SyncErrorCode::Unauthenticated.into())?;
        let user_id = session.user_id().ok_or(Error::UnauthSession)?;
        let srv = self.s.services();
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

    #[tracing::instrument(level = "debug", skip(self), fields(id = self.get_id().to_string()))]
    pub async fn queue_message(
        &mut self,
        msg: Box<MessageSync>,
        nonce: Option<String>,
    ) -> Result<()> {
        let session = match &self.state {
            ConnectionState::Authenticated { session }
            | ConnectionState::Disconnected { session } => session.clone(),
            _ => return Ok(()),
        };

        match &self.state {
            ConnectionState::Disconnected { .. } if !self.queue.can_resume() => {
                self.s.services.connections.live.remove(&self.id);
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
                // FIXME: dont fetch from db for ephemeral messages
                MessageSync::MessageCreate { message } => MessageSync::MessageCreate {
                    message: srv
                        .messages
                        .get(message.channel_id, message.id, session.user_id())
                        .await?,
                },
                MessageSync::MessageUpdate { message } => MessageSync::MessageUpdate {
                    message: srv
                        .messages
                        .get(message.channel_id, message.id, session.user_id())
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
            self.queue.push_sync(msg, nonce);
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self, transport), fields(id = self.get_id().to_string()))]
    pub async fn drain(&mut self, transport: &mut dyn TransportSink) -> Result<()> {
        self.queue.drain(transport, self.id).await
    }

    pub fn get_id(&self) -> ConnectionId {
        self.id
    }

    pub fn state(&self) -> &ConnectionState {
        &self.state
    }
}
