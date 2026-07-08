use std::sync::Arc;
use std::time::Duration;

use axum::extract::WebSocketUpgrade;
use axum::extract::ws::WebSocket;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::any;
use common::v1::types::{MessageClient, MessageEnvelope, MessagePayload, SyncParams};
use futures_util::StreamExt;
use tracing::error;
use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;
use crate::services::connections::Hello;
use crate::sync::transport::{Transport, TransportEvent, WebsocketTransport, WrapperTransport};

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

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().route("/sync", any(sync))
}
