use std::sync::Arc;
use std::time::Duration;

use axum::extract::WebSocketUpgrade;
use axum::extract::ws::WebSocket;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::any;
use common::v1::types::error::SyncErrorCode;
use common::v1::types::presence::Presence;
use common::v1::types::{MessageClient, MessageEnvelope, MessagePayload, SyncParams};
use futures_util::StreamExt;
use lamprey_backend_core::Error;
use tracing::{debug, error, trace, warn};
use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;
use crate::services::connections::Hello;
use crate::sync::transport::{
    Transport, TransportEvent, TransportSink, WebsocketTransport, WrapperTransport,
};

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
async fn worker(s: Arc<ServerState>, params: SyncParams, ws: WebSocket) {
    // PERF: requiring boxing is probably fine but technically slower than it could be
    let (mut send, mut recv) = Box::new(WebsocketTransport::new(ws, params)).split();

    // NOTE: do i need to set up a minimal impl of the protocol here?
    let msg = tokio::time::timeout(Duration::from_secs(5), recv.next()).await;

    match msg {
        // outer result: tokio timeout
        // option: client not sending any more messages
        // inner result: transport errors
        Ok(Some(Ok(TransportEvent::Message(MessageClient::Hello {
            token,
            resume,
            presence,
        })))) => {
            let srv = s.services();

            let seq = if let Some(resume) = &resume {
                resume.seq
            } else {
                0
            };

            let handle = if let Some(resume) = &resume {
                match srv.connections.get(resume.conn) {
                    Some(h) => h,
                    None => {
                        // TODO: better error handling (avoid unwraps)
                        // TODO: create sync error code for expired connection
                        send.send(MessageEnvelope {
                            payload: MessagePayload::Error {
                                error: "expired or invalid connection".into(),
                                code: None,
                            },
                        })
                        .await
                        .unwrap();
                        send.send(MessageEnvelope {
                            payload: MessagePayload::Reconnect { can_resume: false },
                        })
                        .await
                        .unwrap();
                        send.close().await.unwrap();
                        return;
                    }
                }
            } else {
                match srv
                    .connections
                    .accept(Hello {
                        token,
                        resume,
                        presence,
                    })
                    .await
                {
                    Ok(h) => h,
                    Err(err) => {
                        error!("failed to accept connection: {err}");
                        return;
                    }
                }
            };

            let transport = Box::new(WrapperTransport::new(send, recv));
            handle.attach(transport, seq);
        }
        _ => {
            // TODO: better error handling
            send.close().await.unwrap();
        }
    }
}

// TEMP: old sync worker code for reference, will be cleaned up soon
#[cfg(any())]
mod old {
    // TODO: use a tracing span for worker
    // TODO: softer errors for more stuff
    #[tracing::instrument(skip(s, ws))]
    async fn worker(s: Arc<ServerState>, params: SyncParams, ws: WebSocket) {
        let (mut transport, mut client_messages) = WebsocketTransport::new(ws, params).split();

        let mut timeout = Timeout::for_ping();
        let mut sushi = s.inner.subscribe_sushi().await.unwrap();
        let mut conn = Connection::new(s.clone(), params);
        let mut normal_close = false;

        loop {
            tokio::select! {
                sub_res = conn.subscriptions.poll() => {
                    match sub_res {
                        Ok(msg) => {
                            if let Err(err) = conn.queue_message(Box::new(msg), None).await {
                                error!("failed to queue subscription message: {err}");
                            }
                        }
                        Err(err) => {
                            error!("subscription poll error: {err}");
                            let err_str: String = err.to_string();
                            if let Err(send_err) = transport.send(MessageEnvelope {
                                payload: MessagePayload::Error { error: err_str, code: None },
                            }).await {
                                error!("failed to send error message: {send_err}");
                            }
                            if let Err(err) = conn.drain(&mut *transport).await {
                                error!("failed to drain messages on error: {err}");
                            }
                            if let Err(err) = transport.close().await {
                                error!("failed to close websocket: {err}");
                            }
                            break;
                        }
                    }
                }
                ws_msg = client_messages.next() => {
                    match ws_msg {
                        Some(Ok(TransportEvent::Closed(clean))) => {
                            normal_close = clean;
                            break;
                        }
                        Some(Ok(TransportEvent::Message(ws_msg))) => {
                            if let Err(err) = conn.handle_message_client(ws_msg, &mut *transport, &mut timeout).await {
                                error!("error handling websocket message: {err}");

                                if let Err(err) = conn.drain(&mut *transport).await {
                                    error!("failed to drain messages on error: {err}");
                                }
                                if let Err(err) = transport.close().await {
                                    error!("failed to close websocket: {err}");
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
                        debug!("failed to queue sushi message (likely auth check failure): {err}");
                    }
                }
                _ = tokio::time::sleep_until(timeout.get_instant()) => {
                    if !handle_timeout(&mut timeout, &mut *transport, &mut conn).await {
                        warn!("connection timeout, sending reconnect");
                        if let Err(send_err) = transport.send(MessageEnvelope {
                            payload: MessagePayload::Reconnect { can_resume: true },
                        }).await {
                            error!("failed to send reconnect message: {send_err}");
                        }
                        if let Err(err) = conn.drain(&mut *transport).await {
                            error!("failed to drain messages on timeout: {err}");
                        }
                        if let Err(err) = transport.close().await {
                            error!("failed to close websocket on timeout: {err}");
                        }
                        break;
                    }
                }
            }
            if let Err(err) = conn.drain(&mut *transport).await {
                error!("failed to drain messages: {err}");
            } else {
                trace!("did a sync loop");
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
                    warn!("failed to set user {user_id} as offline: {err}");
                }
            }
        }

        conn.disconnect().await;
        debug!("dehydrating syncer: {}", conn.get_id());
        s.services.connections.live.insert(conn.get_id(), conn);
    }

    async fn handle_timeout(
        timeout: &mut Timeout,
        transport: &mut dyn TransportSink,
        conn: &mut Connection,
    ) -> bool {
        match timeout {
            Timeout::Ping(_) => {
                let ping = MessageEnvelope {
                    payload: MessagePayload::Ping {},
                };
                if let Err(err) = transport.send(ping).await {
                    error!("failed to send ping: {err}");
                    return false;
                }
                if let Err(err) = conn.drain(transport).await {
                    error!("failed to drain messages after ping: {err}");
                }
                *timeout = Timeout::for_close();
                true
            }
            Timeout::Close(_) => {
                if let Err(err) = transport.close().await {
                    error!("failed to close websocket on timeout close: {err}");
                }
                false
            }
        }
    }
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().route("/sync", any(sync))
}
