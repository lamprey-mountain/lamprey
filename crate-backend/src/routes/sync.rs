use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::routing::any;
use futures_util::SinkExt;
use tracing::{debug, error};
use types::{MessageEnvelope, MessagePayload};
use utoipa_axum::router::OpenApiRouter;

use crate::error::Error;
use crate::sync::{Connection, Timeout};
use crate::ServerState;

type WsMessage = axum::extract::ws::Message;

/// Sync init
///
/// Open a websocket to start syncing
#[utoipa::path(
    get,
    path = "/sync",
    tags = ["sync"],
    responses(
        (status = UPGRADE_REQUIRED, description = "success"),
    )
)]
async fn sync(State(s): State<Arc<ServerState>>, upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(move |ws| worker(s, ws))
}

#[tracing::instrument(skip(s, ws))]
async fn worker(s: Arc<ServerState>, mut ws: WebSocket) {
    let mut timeout = Timeout::for_ping();
    let mut sushi = s.sushi.subscribe();
    let mut conn = Connection::new(s.clone());

    loop {
        tokio::select! {
            ws_msg = ws.recv() => {
                match ws_msg {
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(ws_msg)) => {
                        if let Err(err) = conn.handle_message(ws_msg, &mut ws, &mut timeout).await {
                            let _ = ws.send(err.into()).await;
                            let _ = ws
                                .send(Message::text(serde_json::to_string(&MessageEnvelope {
                                    payload: MessagePayload::Reconnect { can_resume: true },
                                }).expect("can always serialize message")))
                                .await;
                            let _ = ws.close().await;
                            break;
                        }
                    },
                    _ => break,
                }
            }
            Ok(msg) = sushi.recv() => {
                if let Err(err) = conn.queue_message(msg).await {
                    error!("{err}");
                }
            }
            _ = tokio::time::sleep_until(timeout.get_instant()) => {
                if !handle_timeout(&mut timeout, &mut ws).await {
                    let _ = ws.send(Error::BadStatic("connection timed out").into()).await;
                    let _ = ws
                        .send(Message::text(serde_json::to_string(&MessageEnvelope {
                            payload: MessagePayload::Reconnect { can_resume: true },
                        }).expect("can always serialize message")))
                        .await;
                    let _ = ws.close();
                    break;
                }
            }
        }
        let _ = conn.drain(&mut ws).await;
    }

    conn.disconnect();
    debug!("inserting syncer: {}", conn.get_id());
    s.syncers.insert(conn.get_id().to_owned(), conn);
}

async fn handle_timeout(timeout: &mut Timeout, ws: &mut WebSocket) -> bool {
    match timeout {
        Timeout::Ping(_) => {
            let ping = MessageEnvelope {
                payload: MessagePayload::Ping {},
            };
            let _ = ws.send(serialize(&ping)).await;
            *timeout = Timeout::for_close();
            true
        }
        Timeout::Close(_) => {
            let _ = ws.close().await;
            false
        }
    }
}

fn serialize(msg: &MessageEnvelope) -> WsMessage {
    WsMessage::text(
        serde_json::to_string(msg).expect("server messages should always be able to be serialized"),
    )
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().route("/sync", any(sync))
}
