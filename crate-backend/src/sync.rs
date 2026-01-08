use common::v1::types::{
    sync::{SyncCompression, SyncParams},
    voice::VoiceStateScreenshare,
};
use flate2::{
    write::{ZlibDecoder, ZlibEncoder},
    Compression as FlateCompression,
};
use std::io::Write;
use std::time::Duration;
use std::{collections::VecDeque, sync::Arc};

use axum::extract::ws::{Message, WebSocket};
use common::v1::types::emoji::EmojiOwner;
use common::v1::types::error::SyncError;
use common::v1::types::presence::Presence;
use common::v1::types::util::Time;
use common::v1::types::voice::{SfuCommand, SfuPermissions, SignallingMessage, VoiceState};
use common::v1::types::{self, SERVER_ROOM_ID};
use common::v1::types::{
    InviteTarget, InviteTargetId, MessageClient, MessageEnvelope, MessageSync, Permission, Session,
};
use tokio::time::Instant;
use tracing::{debug, error, trace, warn};

use crate::error::{Error, Result};
use crate::services::members::{MemberListSyncer, MemberListTarget};
use crate::sync::permissions::AuthCheck;
use crate::ServerState;

mod permissions;

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
    id: String,
    pub member_list: Box<MemberListSyncer>,
    compression: Option<Compression>,
}

pub enum Compression {
    Deflate {
        encoder: ZlibEncoder<Vec<u8>>,
        decoder: ZlibDecoder<Vec<u8>>,
    },
}

#[derive(Debug, Clone)]
enum ConnectionState {
    Unauthed,
    Authenticated { session: Session },
    Disconnected { session: Session },
}

impl Connection {
    pub fn new(s: Arc<ServerState>, params: SyncParams) -> Self {
        let compression = match params.compression {
            Some(SyncCompression::Deflate) => Some(Compression::Deflate {
                encoder: ZlibEncoder::new(Vec::new(), FlateCompression::default()),
                decoder: ZlibDecoder::new(Vec::new()),
            }),
            None => None,
        };

        Self {
            state: ConnectionState::Unauthed,
            queue: VecDeque::new(),
            seq_server: 0,
            seq_client: 0,
            id: format!("{}", uuid::Uuid::new_v4().hyphenated()),
            member_list: Box::new(s.services().members.create_syncer()),
            compression,
            s,
        }
    }

    pub fn disconnect(&mut self) {
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

    pub async fn handle_message(
        &mut self,
        ws_msg: Message,
        ws: &mut WebSocket,
        timeout: &mut Timeout,
    ) -> Result<()> {
        let msg = match ws_msg {
            Message::Text(utf8_bytes) => {
                if self.compression.is_some() {
                    return Err(Error::BadStatic(
                        "expected binary message for compressed session",
                    ));
                }
                serde_json::from_str::<MessageClient>(&utf8_bytes)?
            }
            Message::Binary(bytes) => {
                if let Some(Compression::Deflate { decoder, .. }) = &mut self.compression {
                    decoder.write_all(&bytes)?;
                    decoder.flush()?;
                    let decompressed = decoder.get_mut();
                    let msg = serde_json::from_slice::<MessageClient>(decompressed)?;
                    decompressed.clear();
                    msg
                } else {
                    return Err(Error::BadStatic(
                        "unexpected binary message for uncompressed session",
                    ));
                }
            }
            _ => return Ok(()),
        };
        self.handle_message_client(msg, ws, timeout).await
    }

    #[tracing::instrument(level = "debug", skip(self, ws, timeout), fields(id = self.get_id()))]
    pub async fn handle_message_client(
        &mut self,
        msg: MessageClient,
        ws: &mut WebSocket,
        timeout: &mut Timeout,
    ) -> Result<()> {
        trace!("{:#?}", msg);
        match msg {
            MessageClient::Hello {
                token,
                resume: reconnect,
                presence,
            } => {
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

                let msg = MessageEnvelope {
                    payload: types::MessagePayload::Ready {
                        user: Box::new(user),
                        session: session.clone(),
                        conn: self.get_id().to_owned(),
                        seq: 0,
                    },
                };

                ws.send(self.serialize_and_compress(&msg)?).await?;

                self.seq_server += 1;

                if let Some(user_id) = session.user_id() {
                    // send typing states
                    let typing_states = srv.channels.typing_list();
                    for (channel_id, typing_user_id, until) in typing_states {
                        if let Ok(perms) = srv.perms.for_channel(user_id, channel_id).await {
                            if perms.has(Permission::ViewChannel) {
                                self.push_sync(MessageSync::ChannelTyping {
                                    channel_id,
                                    user_id: typing_user_id,
                                    until: until.into(),
                                }, None);
                            }
                        }
                    }

                    // send voice states
                    let voice_states = srv.voice.state_list();
                    for voice_state in voice_states {
                        if let Ok(perms) =
                            srv.perms.for_channel(user_id, voice_state.channel_id).await
                        {
                            let is_ours = self.state.session().and_then(|s| s.user_id())
                                == Some(voice_state.user_id);
                            if perms.has(Permission::ViewChannel) || is_ours {
                                let mut voice_state = voice_state.clone();
                                if !is_ours {
                                    voice_state.session_id = None;
                                }
                                self.push_sync(MessageSync::VoiceState {
                                    user_id: voice_state.user_id,
                                    state: Some(voice_state),
                                    old_state: None,
                                }, None);
                            }
                        }
                    }
                }

                self.member_list.set_user_id(session.user_id()).await;
                self.state = ConnectionState::Authenticated { session };
            }
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

                let target = if let Some(room_id) = room_id {
                    let _perms = srv.perms.for_room(user_id, room_id).await?;
                    Some(MemberListTarget::Room(room_id))
                } else if let Some(thread_id) = thread_id {
                    let perms = srv.perms.for_channel(user_id, thread_id).await?;
                    perms.ensure(Permission::ViewChannel)?;
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
            MessageClient::VoiceDispatch {
                user_id: _,
                payload,
            } => {
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
                        perms.ensure(Permission::ViewChannel)?;
                        perms.ensure(Permission::VoiceConnect)?;
                        let thread = srv.channels.get(state.channel_id, Some(user_id)).await?;
                        if thread.archived_at.is_some() {
                            return Err(Error::BadStatic("thread is archived"));
                        }
                        if thread.deleted_at.is_some() {
                            return Err(Error::BadStatic("thread is removed"));
                        }
                        if thread.locked && !perms.can_use_locked_threads() {
                            return Err(Error::MissingPermissions);
                        }
                        let old_state = srv.voice.state_get(user_id);
                        let mut state = VoiceState {
                            user_id,
                            channel_id: state.channel_id,
                            session_id: Some(session.id),
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
                        if let Err(err) = self.s.sushi_sfu.send(SfuCommand::VoiceState {
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
                        if let Err(err) = self.s.sushi_sfu.send(SfuCommand::VoiceState {
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

                if let Err(err) = self.s.sushi_sfu.send(SfuCommand::Signalling {
                    user_id,
                    inner: payload,
                }) {
                    error!("failed to send to sushi_sfu: {err}");
                }
            }
        }
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self), fields(id = self.get_id()))]
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

        let auth_check = match &*msg {
            MessageSync::RoomCreate { room } => AuthCheck::Room(room.id),
            MessageSync::RoomUpdate { room } => AuthCheck::Room(room.id),
            MessageSync::RoomDelete { room_id } => AuthCheck::Room(*room_id),
            MessageSync::ChannelCreate { channel } => AuthCheck::Channel(channel.id),
            MessageSync::ChannelUpdate { channel } => AuthCheck::Channel(channel.id),
            MessageSync::MessageCreate { message } => AuthCheck::Channel(message.channel_id),
            MessageSync::MessageUpdate { message } => AuthCheck::Channel(message.channel_id),
            MessageSync::UserCreate { user } => AuthCheck::UserMutual(user.id),
            MessageSync::UserUpdate { user } => AuthCheck::UserMutual(user.id),
            MessageSync::PresenceUpdate { user_id, .. } => AuthCheck::UserMutual(*user_id),
            MessageSync::UserConfigGlobal { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::UserConfigRoom { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::UserConfigChannel { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::UserConfigUser { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::RoomMemberCreate { member } => {
                AuthCheck::RoomOrUser(member.room_id, member.user_id)
            }
            MessageSync::RoomMemberUpdate { member } => {
                AuthCheck::RoomOrUser(member.room_id, member.user_id)
            }
            MessageSync::RoomMemberDelete { room_id, user_id } => {
                AuthCheck::RoomOrUser(*room_id, *user_id)
            }
            MessageSync::RoomMemberUpsert { member } => {
                AuthCheck::RoomOrUser(member.room_id, member.user_id)
            }
            MessageSync::ThreadMemberUpsert { member } => {
                AuthCheck::ChannelOrUser(member.thread_id, member.user_id)
            }
            MessageSync::SessionCreate {
                session: upserted_session,
            } => {
                if session.id == upserted_session.id {
                    session = upserted_session.to_owned();
                    self.state = ConnectionState::Authenticated {
                        session: upserted_session.to_owned(),
                    };
                }
                AuthCheck::Custom(session.can_see(upserted_session))
            }
            MessageSync::SessionUpdate {
                session: upserted_session,
            } => {
                if session.id == upserted_session.id {
                    session = upserted_session.to_owned();
                    self.state = ConnectionState::Authenticated {
                        session: upserted_session.to_owned(),
                    };
                }
                AuthCheck::Custom(session.can_see(upserted_session))
            }
            MessageSync::RoleCreate { role } => AuthCheck::Room(role.room_id),
            MessageSync::RoleUpdate { role } => AuthCheck::Room(role.room_id),
            // FIXME(#612): only return invite events to creator and members with InviteManage
            MessageSync::InviteCreate { invite } => match &invite.invite.target {
                InviteTarget::Room { room, channel: _ } => AuthCheck::Room(room.id),
                InviteTarget::Gdm { channel, .. } => AuthCheck::Channel(channel.id),
                InviteTarget::Server => {
                    AuthCheck::RoomPerm(SERVER_ROOM_ID, Permission::ServerOversee)
                }
                InviteTarget::User { user, .. } => AuthCheck::User(user.id),
            },
            MessageSync::InviteUpdate { invite } => match &invite.invite.target {
                InviteTarget::Room { room, .. } => AuthCheck::Room(room.id),
                InviteTarget::Gdm { channel, .. } => AuthCheck::Channel(channel.id),
                InviteTarget::Server => {
                    AuthCheck::RoomPerm(SERVER_ROOM_ID, Permission::ServerOversee)
                }
                InviteTarget::User { user, .. } => AuthCheck::User(user.id),
            },
            MessageSync::MessageDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::MessageVersionDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::UserDelete { id } => AuthCheck::UserMutual(*id),
            MessageSync::SessionDelete { id, user_id } => {
                // TODO: send message when other sessions from the same user are deleted
                if *id == session.id {
                    self.state = ConnectionState::Unauthed;
                    AuthCheck::Custom(true)
                } else if let Some(user_id) = user_id {
                    AuthCheck::User(*user_id)
                } else {
                    AuthCheck::Custom(false)
                }
            }
            MessageSync::SessionDeleteAll { user_id } => AuthCheck::User(*user_id),
            MessageSync::RoleDelete { room_id, .. } => AuthCheck::Room(*room_id),
            MessageSync::RoleReorder { room_id, .. } => AuthCheck::Room(*room_id),
            MessageSync::InviteDelete { target, .. } => match target {
                InviteTargetId::Room { room_id, .. } => AuthCheck::Room(*room_id),
                InviteTargetId::Gdm { channel_id, .. } => AuthCheck::Channel(*channel_id),
                InviteTargetId::Server => {
                    AuthCheck::RoomPerm(SERVER_ROOM_ID, Permission::ServerOversee)
                }
                InviteTargetId::User { user_id, .. } => AuthCheck::User(*user_id),
            },
            MessageSync::ChannelTyping { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::ChannelAck { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::RelationshipUpsert { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::RelationshipDelete { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::ReactionCreate { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::ReactionDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::ReactionDeleteKey { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::ReactionDeleteAll { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::MessageDeleteBulk { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::MessageRemove { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::MessageRestore { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::VoiceDispatch { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::VoiceState {
                state,
                user_id,
                old_state,
            } => match (state, old_state) {
                (None, None) => AuthCheck::User(*user_id),
                (None, Some(o)) => AuthCheck::Channel(o.channel_id),
                (Some(s), None) => AuthCheck::Channel(s.channel_id),
                (Some(s), Some(o)) => AuthCheck::EitherChannel(s.channel_id, o.channel_id),
            },
            MessageSync::EmojiCreate { emoji } => match emoji
                .owner
                .as_ref()
                .expect("emoji sync events from server always has owner")
            {
                EmojiOwner::Room { room_id } => AuthCheck::Room(*room_id),
                EmojiOwner::User => AuthCheck::User(
                    emoji
                        .creator_id
                        .expect("emoji sync events from server always has creator_id"),
                ),
            },
            MessageSync::EmojiUpdate { emoji } => match emoji
                .owner
                .as_ref()
                .expect("emoji sync events from server always has owner")
            {
                EmojiOwner::Room { room_id } => AuthCheck::Room(*room_id),
                EmojiOwner::User => AuthCheck::User(
                    emoji
                        .creator_id
                        .expect("emoji sync events from server always has creator_id"),
                ),
            },
            MessageSync::EmojiDelete {
                room_id,
                emoji_id: _,
            } => AuthCheck::Room(*room_id),
            MessageSync::ConnectionCreate { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::ConnectionDelete { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::AuditLogEntryCreate { entry } => {
                AuthCheck::RoomPerm(entry.room_id, Permission::ViewAuditLog)
            }
            MessageSync::BanCreate { room_id, .. } => {
                AuthCheck::RoomPerm(*room_id, Permission::MemberBan)
            }
            MessageSync::BanDelete { room_id, .. } => {
                AuthCheck::RoomPerm(*room_id, Permission::MemberBan)
            }
            MessageSync::AutomodRuleCreate { rule } => {
                AuthCheck::RoomPerm(rule.room_id, Permission::RoomManage)
            }
            MessageSync::AutomodRuleUpdate { rule } => {
                AuthCheck::RoomPerm(rule.room_id, Permission::RoomManage)
            }
            MessageSync::AutomodRuleDelete { room_id, .. } => {
                AuthCheck::RoomPerm(*room_id, Permission::RoomManage)
            }
            MessageSync::AutomodRuleExecute { execution } => {
                AuthCheck::RoomPerm(execution.rule.room_id, Permission::RoomManage)
            }
            MessageSync::MemberListSync { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::InboxNotificationCreate { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::InboxMarkRead { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::InboxMarkUnread { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::InboxFlush { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::CalendarEventCreate { event } => AuthCheck::Channel(event.channel_id),
            MessageSync::CalendarEventUpdate { event } => AuthCheck::Channel(event.channel_id),
            MessageSync::CalendarEventDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::CalendarOverwriteCreate { channel_id, .. } => {
                AuthCheck::Channel(*channel_id)
            }
            MessageSync::CalendarOverwriteUpdate { channel_id, .. } => {
                AuthCheck::Channel(*channel_id)
            }
            MessageSync::CalendarOverwriteDelete { channel_id, .. } => {
                AuthCheck::Channel(*channel_id)
            }
            MessageSync::CalendarRsvpCreate { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::CalendarRsvpDelete { channel_id, .. } => AuthCheck::Channel(*channel_id),
            MessageSync::CalendarOverwriteRsvpCreate { channel_id, .. } => {
                AuthCheck::Channel(*channel_id)
            }
            MessageSync::CalendarOverwriteRsvpDelete { channel_id, .. } => {
                AuthCheck::Channel(*channel_id)
            }
            MessageSync::WebhookCreate { webhook } => {
                AuthCheck::ChannelPerm(webhook.channel_id, Permission::IntegrationsManage)
            }
            MessageSync::WebhookUpdate { webhook } => {
                AuthCheck::ChannelPerm(webhook.channel_id, Permission::IntegrationsManage)
            }
            MessageSync::WebhookDelete { channel_id, .. } => {
                AuthCheck::ChannelPerm(*channel_id, Permission::IntegrationsManage)
            }
            MessageSync::RatelimitUpdate { user_id, .. } => AuthCheck::User(*user_id),
        };
        let should_send = auth_check.should_send(&session, &self.s).await?;
        if should_send {
            let d = self.s.data();
            let srv = self.s.services();
            let msg = match *msg {
                MessageSync::ChannelCreate { channel } => MessageSync::ChannelCreate {
                    channel: Box::new(srv.channels.get(channel.id, session.user_id()).await?),
                },
                MessageSync::ChannelUpdate { channel } => MessageSync::ChannelUpdate {
                    channel: Box::new(srv.channels.get(channel.id, session.user_id()).await?),
                },
                MessageSync::MessageCreate { message } => MessageSync::MessageCreate {
                    message: {
                        let mut m = d
                            .message_get(message.channel_id, message.id, session.user_id().unwrap())
                            .await?;
                        self.s.presign_message(&mut m).await?;
                        // FIXME: include nonce
                        m
                    },
                },
                MessageSync::MessageUpdate { message } => MessageSync::MessageUpdate {
                    message: {
                        let mut m = d
                            .message_get(message.channel_id, message.id, session.user_id().unwrap())
                            .await?;
                        self.s.presign_message(&mut m).await?;
                        // FIXME: include nonce
                        m
                    },
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
                        if !perms.has(Permission::ViewChannel) {
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

    #[tracing::instrument(level = "debug", skip(self, ws), fields(id = self.get_id()))]
    pub async fn drain(&mut self, ws: &mut WebSocket) -> Result<()> {
        let last_seen = self.seq_client;
        let mut high_water_mark = last_seen;

        let queue = &self.queue;
        let compression = &mut self.compression;

        for (seq, msg) in queue.iter().rev() {
            if seq.is_none_or(|s| s > last_seen) {
                ws.send(Self::compress_message(compression, msg)?).await?;

                if let Some(seq) = *seq {
                    high_water_mark = high_water_mark.max(seq);
                }
            }
        }
        self.seq_client = high_water_mark;
        self.queue.retain(|(seq, _)| seq.is_some());
        Ok(())
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    fn compress_message(
        compression: &mut Option<Compression>,
        msg: &MessageEnvelope,
    ) -> Result<WsMessage> {
        let json = serde_json::to_string(msg)?;
        Ok(
            if let Some(Compression::Deflate { encoder, .. }) = compression {
                encoder.write_all(json.as_bytes())?;
                encoder.flush()?;
                let compressed = encoder.get_mut();
                let data = compressed.clone();
                compressed.clear();
                WsMessage::Binary(data.into())
            } else {
                WsMessage::text(json)
            },
        )
    }

    fn serialize_and_compress(&mut self, msg: &MessageEnvelope) -> Result<WsMessage> {
        Self::compress_message(&mut self.compression, msg)
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
