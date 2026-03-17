use std::sync::Arc;

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::any;
use common::v1::types::presence::Presence;
use common::v1::types::{MessageEnvelope, MessagePayload, SyncParams};
use futures_util::{SinkExt, StreamExt};
use utoipa_axum::router::OpenApiRouter;

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
    params(SyncParams),
    responses(
        (status = UPGRADE_REQUIRED, description = "success"),
    )
)]
async fn sync(
    State(s): State<Arc<ServerState>>,
    Query(params): Query<SyncParams>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    upgrade.on_upgrade(move |ws| worker(s, params, ws))
}

#[tracing::instrument(skip(s, ws))]
async fn worker(s: Arc<ServerState>, params: SyncParams, mut ws: WebSocket) {
    let mut timeout = Timeout::for_ping();
    let mut sushi = s.inner.subscribe_sushi().await.unwrap();
    let mut conn = Connection::new(s.clone(), params);
    let mut normal_close = false;

    loop {
        tokio::select! {
            member_list_res = conn.member_list.poll() => {
                match member_list_res {
                    Ok(msg) => {
                        if let Err(err) = conn.queue_message(Box::new(msg), None).await {
                            tracing::error!("failed to queue member list message: {err}");
                        }
                    }
                    Err(err) => {
                        tracing::error!("member list poll error: {err}");
                        if let Ok(msg) = conn.serialize(&MessageEnvelope {
                            payload: MessagePayload::Error { error: err.to_string(), code: None },
                        }) {
                            if let Err(send_err) = ws.send(msg).await {
                                tracing::error!("failed to send error message: {send_err}");
                            }
                        }
                        if let Err(err) = conn.drain(&mut ws).await {
                            tracing::error!("failed to drain messages on error: {err}");
                        }
                        if let Err(err) = ws.close().await {
                            tracing::error!("failed to close websocket: {err}");
                        }
                        break;
                    }
                }
            }
            doc_res = conn.document.poll() => {
                match doc_res {
                    Ok(msg) => {
                        if let Err(err) = conn.queue_message(Box::new(msg), None).await {
                            tracing::error!("failed to queue document message: {err}");
                        }
                    }
                    Err(err) => {
                        tracing::error!("document poll error: {err}");
                        if let Ok(msg) = conn.serialize(&MessageEnvelope {
                            payload: MessagePayload::Error { error: err.to_string(), code: None },
                        }) {
                            if let Err(send_err) = ws.send(msg).await {
                                tracing::error!("failed to send error message: {send_err}");
                            }
                        }
                        if let Err(err) = conn.drain(&mut ws).await {
                            tracing::error!("failed to drain messages on error: {err}");
                        }
                        if let Err(err) = ws.close().await {
                            tracing::error!("failed to close websocket: {err}");
                        }
                        break;
                    }
                }
            }
            ws_msg = ws.recv() => {
                match ws_msg {
                    Some(Ok(Message::Close(Some(CloseFrame { code, .. })))) => {
                        // 1000 = Normal Closure, 1001 = Going Away
                        if code == 1000 || code == 1001 {
                            normal_close = true;
                        }
                        break;
                    }
                    Some(Ok(Message::Close(None))) => break,
                    Some(Ok(ws_msg)) => {
                        if let Err(err) = conn.handle_message(ws_msg, &mut ws, &mut timeout).await {
                            tracing::error!("error handling websocket message: {err}");
                            if let Ok(msg) = conn.serialize(&MessageEnvelope {
                                payload: MessagePayload::Error { error: err.to_string(), code: None },
                            }) {
                                if let Err(send_err) = ws.send(msg).await {
                                    tracing::error!("failed to send error message: {send_err}");
                                }
                            }
                            if let Ok(msg) = conn.serialize(&MessageEnvelope {
                                payload: MessagePayload::Reconnect { can_resume: false },
                            }) {
                                if let Err(send_err) = ws.send(msg).await {
                                    tracing::error!("failed to send reconnect message: {send_err}");
                                }
                            }
                            // TODO: don't close ws on *every* error - most are recoverable
                            if let Err(err) = conn.drain(&mut ws).await {
                                tracing::error!("failed to drain messages on error: {err}");
                            }
                            if let Err(err) = ws.close().await {
                                tracing::error!("failed to close websocket: {err}");
                            }
                            break;
                        }
                    },
                    _ => break,
                }
            }
            Some(msg) = sushi.next() => {
                if let Err(err) = conn.queue_message(Box::new(msg.message), msg.nonce).await {
                    // most of the errors that are returned are auth check failures, which we don't need to log
                    tracing::debug!("failed to queue sushi message (likely auth check failure): {err}");
                }
            }
            _ = tokio::time::sleep_until(timeout.get_instant()) => {
                if !handle_timeout(&mut timeout, &mut ws, &mut conn).await {
                    tracing::warn!("connection timeout, sending reconnect");
                    if let Ok(msg) = conn.serialize(&MessageEnvelope {
                        payload: MessagePayload::Reconnect { can_resume: true },
                    }) {
                        if let Err(send_err) = ws.send(msg).await {
                            tracing::error!("failed to send reconnect message: {send_err}");
                        }
                    }
                    if let Err(err) = conn.drain(&mut ws).await {
                        tracing::error!("failed to drain messages on timeout: {err}");
                    }
                    if let Err(err) = ws.close().await {
                        tracing::error!("failed to close websocket on timeout: {err}");
                    }
                    break;
                }
            }
        }
        if let Err(err) = conn.drain(&mut ws).await {
            tracing::error!("failed to drain messages: {err}");
        }
    }

    // mark user as offline on normal close
    if normal_close {
        if let Some(user_id) = conn.state().session().and_then(|s| s.user_id()) {
            if let Err(err) = s
                .services()
                .presence
                .set(user_id, Presence::offline())
                .await
            {
                tracing::warn!("failed to set user {user_id} as offline: {err}");
            }
        }
    }

    conn.disconnect().await;
    tracing::debug!("dehydrating syncer: {}", conn.get_id());
    s.syncers.insert(conn.get_id(), conn);
}

async fn handle_timeout(timeout: &mut Timeout, ws: &mut WebSocket, conn: &mut Connection) -> bool {
    match timeout {
        Timeout::Ping(_) => {
            let ping = MessageEnvelope {
                payload: MessagePayload::Ping {},
            };
            match conn.serialize(&ping) {
                Ok(msg) => {
                    if let Err(err) = ws.send(msg).await {
                        tracing::error!("failed to send ping: {err}");
                        return false;
                    }
                }
                Err(err) => {
                    tracing::error!("failed to serialize ping: {err}");
                    return false;
                }
            }
            if let Err(err) = conn.drain(ws).await {
                tracing::error!("failed to drain messages after ping: {err}");
            }
            *timeout = Timeout::for_close();
            true
        }
        Timeout::Close(_) => {
            if let Err(err) = ws.close().await {
                tracing::error!("failed to close websocket on timeout close: {err}");
            }
            false
        }
    }
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().route("/sync", any(sync))
}
