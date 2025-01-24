use std::time::Duration;
use std::{collections::VecDeque, sync::Arc};

use axum::extract::ws::{Message, WebSocket};
use tokio::time::Instant;
use tracing::debug;
use types::{
    MessageClient, MessageEnvelope, MessageSync, Permission, RoomId, Session, SessionStatus,
    ThreadId,
};

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

    pub async fn handle_message_client(
        &mut self,
        msg: MessageClient,
        ws: &mut WebSocket,
        timeout: &mut Timeout,
    ) -> Result<()> {
        match msg {
            MessageClient::Hello {
                token,
                resume: reconnect,
            } => {
                let data = self.s.data();
                let session =
                    data.session_get_by_token(token)
                        .await
                        .map_err(|err| match err.into() {
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

                let user = match session.user_id() {
                    Some(user_id) => Some(data.user_get(user_id).await?),
                    None => None,
                };
                let msg = MessageEnvelope {
                    payload: types::MessagePayload::Ready {
                        user,
                        conn: self.get_id().to_owned(),
                        seq: 0,
                    },
                };

                ws.send(WsMessage::text(serde_json::to_string(&msg)?))
                    .await?;

                self.seq_server += 1;
                self.state = ConnectionState::Authenticated { session };
            }
            MessageClient::Pong => {
                *timeout = Timeout::Ping(Instant::now() + HEARTBEAT_TIME);
            }
        }
        Ok(())
    }

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
            MessageSync::UpsertUser { user } => {
                // TODO: more user upserts?
                AuthCheck::Custom(session.user_id().is_some_and(|id| user.id == id))
            }
            MessageSync::UpsertMember { member } => AuthCheck::Room(member.room_id),
            MessageSync::UpsertSession {
                session: upserted_session,
            } => {
                if session.id == upserted_session.id {
                    session = upserted_session.to_owned();
                    self.state = ConnectionState::Authenticated { session: upserted_session.to_owned() };
                }
                AuthCheck::Custom(session.can_see(upserted_session))
            },
            MessageSync::UpsertRole { role } => AuthCheck::Room(role.room_id),
            MessageSync::UpsertInvite { invite: _ } => {
                // TODO
                AuthCheck::Custom(false)
            }
            MessageSync::DeleteMessage {
                thread_id,
                message_id: _,
            } => AuthCheck::Thread(*thread_id),
            MessageSync::DeleteMessageVersion {
                thread_id,
                message_id: _,
                version_id: _,
            } => AuthCheck::Thread(*thread_id),
            MessageSync::DeleteUser { id } => {
                // TODO
                AuthCheck::Custom(session.user_id().is_some_and(|i| *id == i))
            }
            MessageSync::DeleteSession { id } => {
                // TODO: send message when other sessions from the same user are deleted
                if *id == session.id {
                    self.state = ConnectionState::Unauthed;
                    AuthCheck::Custom(true)
                } else {
                    AuthCheck::Custom(false)
                }
            }
            MessageSync::DeleteRole {
                room_id,
                role_id: _,
            } => AuthCheck::Room(*room_id),
            MessageSync::DeleteMember {
                room_id,
                user_id: _,
            } => AuthCheck::Room(*room_id),
            MessageSync::DeleteInvite { code: _ } => todo!(),
            MessageSync::Webhook {
                hook_id: _,
                data: _,
            } => {
                todo!()
            }
        };
        let should_send = match (session.user_id(), auth_check) {
            (Some(user_id), AuthCheck::Room(room_id)) => {
                let perms = self.s.data().permission_room_get(user_id, room_id).await?;
                perms.has(Permission::View)
            }
            (Some(user_id), AuthCheck::Thread(thread_id)) => {
                let perms = self
                    .s
                    .data()
                    .permission_thread_get(user_id, thread_id)
                    .await?;
                perms.has(Permission::View)
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

    pub async fn drain(&mut self, ws: &mut WebSocket) -> Result<()> {
        let last_seen = self.seq_client;
        debug!("drain id={} last_seen={}", self.get_id(), last_seen);
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
