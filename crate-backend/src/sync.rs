use std::time::Duration;
use std::{collections::VecDeque, sync::Arc};

use axum::extract::ws::{Message, WebSocket};
use common::v1::types;
use common::v1::types::user_status::Status;
use common::v1::types::{
    InviteTarget, InviteTargetId, MessageClient, MessageEnvelope, MessageSync, Permission, RoomId,
    Session, ThreadId, UserId,
};
use tokio::time::Instant;
use tracing::{debug, trace};

use crate::error::{Error, Result};
use crate::ServerState;

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
}

#[derive(Debug, Clone)]
enum ConnectionState {
    Unauthed,
    Authenticated { session: Session },
    Disconnected { session: Session },
}

#[derive(Debug)]
enum AuthCheck {
    Custom(bool),
    Room(RoomId),
    RoomOrUser(RoomId, UserId),
    ThreadOrUser(ThreadId, UserId),
    User(UserId),
    UserMutual(UserId),
    Thread(ThreadId),
}

impl Connection {
    pub fn new(s: Arc<ServerState>) -> Self {
        Self {
            state: ConnectionState::Unauthed,
            s,
            queue: VecDeque::new(),
            seq_server: 0,
            seq_client: 0,
            id: format!("{}", uuid::Uuid::new_v4().hyphenated()),
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
            Err(Error::BadStatic("too old"))
        }
    }

    pub async fn handle_message(
        &mut self,
        ws_msg: Message,
        ws: &mut WebSocket,
        timeout: &mut Timeout,
    ) -> Result<()> {
        match ws_msg {
            Message::Text(utf8_bytes) => {
                let msg: MessageClient = serde_json::from_str(&utf8_bytes)?;
                self.handle_message_client(msg, ws, timeout).await
            }
            Message::Binary(_) => Err(Error::BadStatic("doesn't support binary sorry")),
            _ => Ok(()),
        }
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
                status,
            } => {
                let srv = self.s.services();
                let session = srv
                    .sessions
                    .get_by_token(token)
                    .await
                    .map_err(|err| match err {
                        Error::NotFound => Error::MissingAuth,
                        other => other,
                    })?;

                // TODO: more forgiving reconnections
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

                let user = if let Some(user_id) = session.user_id() {
                    let srv = self.s.services();
                    let user = srv
                        .users
                        .status_set(
                            user_id,
                            status
                                .map(|s| s.apply(Status::offline()))
                                .unwrap_or(Status::online()),
                        )
                        .await?;
                    Some(user)
                } else {
                    None
                };

                let msg = MessageEnvelope {
                    payload: types::MessagePayload::Ready {
                        user,
                        session: session.clone(),
                        conn: self.get_id().to_owned(),
                        seq: 0,
                    },
                };

                ws.send(WsMessage::text(serde_json::to_string(&msg)?))
                    .await?;

                self.seq_server += 1;
                self.state = ConnectionState::Authenticated { session };
            }
            MessageClient::Status { status } => {
                let session = match &self.state {
                    ConnectionState::Unauthed => return Err(Error::MissingAuth),
                    ConnectionState::Authenticated { session } => session,
                    ConnectionState::Disconnected { .. } => {
                        panic!("somehow recv msg while disconnected?")
                    }
                };
                let srv = self.s.services();
                let user_id = session.user_id().ok_or(Error::UnauthSession)?;
                srv.users
                    .status_set(user_id, status.apply(Status::offline()))
                    .await?;
            }
            MessageClient::Pong => {
                let session = match &self.state {
                    ConnectionState::Unauthed => return Err(Error::MissingAuth),
                    ConnectionState::Authenticated { session } => session,
                    ConnectionState::Disconnected { .. } => {
                        panic!("somehow recv msg while disconnected?")
                    }
                };
                let srv = self.s.services();
                let user_id = session.user_id().ok_or(Error::UnauthSession)?;
                srv.users.status_ping(user_id).await?;
                *timeout = Timeout::Ping(Instant::now() + HEARTBEAT_TIME);
            }
        }
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self), fields(id = self.get_id()))]
    pub async fn queue_message(&mut self, msg: MessageSync) -> Result<()> {
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

        let auth_check = match &msg {
            MessageSync::UpsertRoom { room } => AuthCheck::Room(room.id),
            MessageSync::UpsertThread { thread } => AuthCheck::Thread(thread.id),
            MessageSync::UpsertMessage { message } => AuthCheck::Thread(message.thread_id),
            MessageSync::UpsertUser { user } => AuthCheck::UserMutual(user.id),
            MessageSync::UpsertRoomMember { member } => {
                AuthCheck::RoomOrUser(member.room_id, member.user_id)
            }
            MessageSync::UpsertThreadMember { member } => {
                AuthCheck::ThreadOrUser(member.thread_id, member.user_id)
            }
            MessageSync::UpsertSession {
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
            MessageSync::UpsertRole { role } => AuthCheck::Room(role.room_id),
            MessageSync::UpsertInvite { invite } => match &invite.invite.target {
                InviteTarget::User { user } => AuthCheck::User(user.id),
                InviteTarget::Room { room } => AuthCheck::Room(room.id),
                InviteTarget::Thread { thread, .. } => AuthCheck::Thread(thread.id),
            },
            MessageSync::DeleteMessage { thread_id, .. } => AuthCheck::Thread(*thread_id),
            MessageSync::DeleteMessageVersion { thread_id, .. } => AuthCheck::Thread(*thread_id),
            MessageSync::DeleteUser { id } => AuthCheck::UserMutual(*id),
            MessageSync::DeleteSession { id, user_id } => {
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
            MessageSync::DeleteRole { room_id, .. } => AuthCheck::Room(*room_id),
            MessageSync::DeleteInvite { target, .. } => match target {
                InviteTargetId::User { user_id } => {
                    AuthCheck::Custom(session.user_id().is_some_and(|id| id == *user_id))
                }
                InviteTargetId::Room { room_id } => AuthCheck::Room(*room_id),
                InviteTargetId::Thread { thread_id, .. } => AuthCheck::Thread(*thread_id),
            },
            MessageSync::Typing { thread_id, .. } => AuthCheck::Thread(*thread_id),
            MessageSync::ThreadAck { .. } => todo!(),
            MessageSync::RelationshipUpsert { user_id, .. } => AuthCheck::User(*user_id),
            MessageSync::RelationshipDelete { user_id } => AuthCheck::User(*user_id),
            MessageSync::ReactionMessageUpsert { thread_id, .. } => AuthCheck::Thread(*thread_id),
            MessageSync::ReactionMessageRemove { thread_id, .. } => AuthCheck::Thread(*thread_id),
            MessageSync::ReactionMessagePurge { thread_id, .. } => AuthCheck::Thread(*thread_id),
            MessageSync::ReactionThreadUpsert { thread_id, .. } => AuthCheck::Thread(*thread_id),
            MessageSync::ReactionThreadRemove { thread_id, .. } => AuthCheck::Thread(*thread_id),
            MessageSync::ReactionThreadPurge { thread_id, .. } => AuthCheck::Thread(*thread_id),
            MessageSync::MessageDeleteBulk { thread_id, .. } => AuthCheck::Thread(*thread_id),
        };
        let should_send = match (session.user_id(), auth_check) {
            (Some(user_id), AuthCheck::Room(room_id)) => {
                let perms = self.s.services().perms.for_room(user_id, room_id).await?;
                perms.has(Permission::View)
            }
            (Some(auth_user_id), AuthCheck::RoomOrUser(room_id, target_user_id)) => {
                if auth_user_id == target_user_id {
                    true
                } else {
                    let perms = self
                        .s
                        .services()
                        .perms
                        .for_room(auth_user_id, room_id)
                        .await?;
                    perms.has(Permission::View)
                }
            }
            (Some(user_id), AuthCheck::Thread(thread_id)) => {
                let perms = self
                    .s
                    .services()
                    .perms
                    .for_thread(user_id, thread_id)
                    .await?;
                perms.has(Permission::View)
            }
            (Some(auth_user_id), AuthCheck::ThreadOrUser(thread_id, target_user_id)) => {
                if auth_user_id == target_user_id {
                    true
                } else {
                    let perms = self
                        .s
                        .services()
                        .perms
                        .for_thread(auth_user_id, thread_id)
                        .await?;
                    perms.has(Permission::View)
                }
            }
            (Some(auth_user_id), AuthCheck::User(target_user_id)) => auth_user_id == target_user_id,
            (Some(auth_user_id), AuthCheck::UserMutual(target_user_id)) => {
                if auth_user_id == target_user_id {
                    true
                } else {
                    self.s
                        .services()
                        .perms
                        .is_mutual(auth_user_id, target_user_id)
                        .await?
                }
            }
            (_, AuthCheck::Custom(b)) => b,
            (None, _) => false,
        };
        if should_send {
            self.push_sync(msg);
        }
        Ok(())
    }

    fn push_sync(&mut self, sync: MessageSync) {
        let seq = self.seq_server;
        let msg = MessageEnvelope {
            payload: types::MessagePayload::Sync { data: sync, seq },
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
        for (seq, msg) in self.queue.iter().rev() {
            if seq.is_none_or(|s| s > last_seen) {
                let json = serde_json::to_string(&msg)?;
                ws.send(WsMessage::text(json)).await?;
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
