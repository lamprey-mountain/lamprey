use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::routing::any;
use futures_util::SinkExt;
use tokio::time::Instant;
use utoipa_axum::router::OpenApiRouter;

use crate::types::{MessageClient, MessageServer, Permission, RoomId, Session, ThreadId};
use crate::ServerState;

use crate::error::{Error, Result};

/// Sync init
///
/// Open a websocket to start syncing
#[utoipa::path(
    get,
    path = "/sync",
    tags = ["invite"],
    responses(
        (status = UPGRADE_REQUIRED, description = "success"),
    )
)]
async fn sync(State(s): State<ServerState>, upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(move |ws| worker(s, ws))
}

enum ClientState {
    Unauthed,
    Authenticated { session: Session },
    // Closed,
}

enum Timeout {
    Ping(Instant),
    Close(Instant),
}

impl Timeout {
    fn get_instant(&self) -> Instant {
        match self {
            Timeout::Ping(instant) => *instant,
            Timeout::Close(instant) => *instant,
        }
    }
}

const HEARTBEAT_TIME: Duration = Duration::from_secs(30);

async fn worker(s: ServerState, mut ws: WebSocket) {
    let mut state = ClientState::Unauthed;
    let mut timeout = Timeout::Ping(Instant::now() + HEARTBEAT_TIME);
    // let mut client = Client::new(s, ws);
    let mut sushi = s.sushi.subscribe();
    loop {
        tokio::select! {
            ws_msg = ws.recv() => {
                match ws_msg {
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(ws_msg)) => {
                        if let Err(err) = handle_message(ws_msg, &mut ws, &mut timeout, &s, &mut state).await {
                            let _ = ws.send(err.into()).await;
                            let _ = ws.close().await;
                            break;
                        }
                    },
                    _ => break,
                }
            }
            Ok(msg) = sushi.recv() => {
                if let Err(err) = handle_sushi(msg, &mut ws, &s, &mut state).await {
                    let _ = ws.send(err.into()).await;
                    let _ = ws.close().await;
                    break;
                }
            }
            _ = tokio::time::sleep_until(timeout.get_instant()) => {
                if !handle_timeout(&mut timeout, &mut ws).await {
                    break;
                }
            }
        }
    }
}

async fn handle_message(
    ws_msg: Message,
    ws: &mut WebSocket,
    timeout: &mut Timeout,
    s: &ServerState,
    state: &mut ClientState,
) -> Result<()> {
    match ws_msg {
        Message::Text(utf8_bytes) => {
            let msg: MessageClient = serde_json::from_str(&utf8_bytes)?;
            match msg {
                // TODO: resuming
                MessageClient::Hello { token, last_id: _ } => {
                    let data = s.data();
                    let session = data.session_get_by_token(&token).await?;
                    // if session.status == SessionStatus::Unauthorized {
                    //     return Err(Error::UnauthSession)
                    // }
                    let user = data.user_get(session.user_id).await?;
                    ws.send(MessageServer::Ready { user }.into()).await?;
                    *state = ClientState::Authenticated { session };
                }
                MessageClient::Pong => {
                    *timeout = Timeout::Ping(Instant::now() + HEARTBEAT_TIME);
                }
            }
        }
        Message::Binary(_) => {
            return Err(Error::BadStatic("doesn't support binary sorry"));
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug)]
enum AuthCheck {
    Custom(bool),
    Room(RoomId),
    Thread(ThreadId),
}

async fn handle_sushi(
    msg: MessageServer,
    ws: &mut WebSocket,
    s: &ServerState,
    state: &mut ClientState,
) -> Result<()> {
    let ClientState::Authenticated { session } = &state else {
        return Ok(());
    };

    let user_id = session.user_id;
    let auth_check = match &msg {
        MessageServer::Ping {} | MessageServer::Ready { .. } | MessageServer::Error { .. } => {
            AuthCheck::Custom(true)
        }
        MessageServer::UpsertRoom { room } => AuthCheck::Room(room.id),
        MessageServer::UpsertThread { thread } => AuthCheck::Thread(thread.id),
        MessageServer::UpsertMessage { message } => AuthCheck::Thread(message.thread_id),
        MessageServer::UpsertUser { user } => {
            // TODO: more user upserts?
            AuthCheck::Custom(user.id == user_id)
        }
        MessageServer::UpsertMember { member } => AuthCheck::Room(member.room_id),
        MessageServer::UpsertSession { session } => AuthCheck::Custom(session.user_id == user_id),
        MessageServer::UpsertRole { role } => AuthCheck::Room(role.room_id),
        MessageServer::UpsertInvite { invite: _ } => {
            // TODO
            AuthCheck::Custom(false)
        }
        MessageServer::DeleteMessage {
            thread_id,
            message_id: _,
        } => AuthCheck::Thread(*thread_id),
        MessageServer::DeleteMessageVersion {
            thread_id,
            message_id: _,
            version_id: _,
        } => AuthCheck::Thread(*thread_id),
        MessageServer::DeleteUser { id } => {
            // TODO
            AuthCheck::Custom(*id == user_id)
        }
        MessageServer::DeleteSession { id: _ } => todo!(),
        MessageServer::DeleteRole { room_id, role_id: _ } => AuthCheck::Room(*room_id),
        MessageServer::DeleteMember { room_id, user_id: _ } => AuthCheck::Room(*room_id),
        MessageServer::DeleteInvite { code: _ } => todo!(),
        MessageServer::Webhook {
            hook_id: _,
            data: _,
        } => {
            todo!()
        }
    };
    let should_send = match auth_check {
        AuthCheck::Room(room_id) => {
            let perms = s.data().permission_room_get(user_id, room_id).await?;
            perms.has(Permission::View)
        }
        AuthCheck::Thread(thread_id) => {
            let perms = s.data().permission_thread_get(user_id, thread_id).await?;
            perms.has(Permission::View)
        }
        AuthCheck::Custom(b) => b,
    };
    if should_send {
        ws.send(msg.into()).await?;
    }
    Ok(())
}

async fn handle_timeout(timeout: &mut Timeout, ws: &mut WebSocket) -> bool {
    match timeout {
        Timeout::Ping(_) => {
            let ping = MessageServer::Ping {};
            let _ = ws.send(ping.into()).await;
            *timeout = Timeout::Close(Instant::now() + Duration::from_secs(10));
            true
        }
        Timeout::Close(_) => {
            let _ = ws.close().await;
            false
        }
    }
}

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new().route("/sync", any(sync))
}
