use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use common::v1::types::voice::{SignallingMessage, VoiceState};
use common::v1::types::{MessageSync, UserId};
use http::HeaderMap;
use tokio::select;
use tracing::error;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::{Error, ServerState};

// TODO: does this count as an implementation detail or should it be moved to common?

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "type")]
pub enum SfuCommand {
    #[cfg(feature = "voice")]
    VoiceDispatch {
        user_id: UserId,
        payload: SignallingMessage,
    },

    #[cfg(feature = "voice")]
    VoiceState {
        user_id: UserId,
        old: Option<VoiceState>,
        state: Option<VoiceState>,
    },
}

/// Internal rpc
#[utoipa::path(
    get,
    path = "/internal/rpc",
    tags = ["internal"],
    responses((status = 101, description = "Switching Protocols")),
)]
#[allow(unused)]
async fn internal_rpc(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let auth = headers
        .get("authorization")
        .ok_or(Error::MissingAuth)?
        .to_str()?;
    if auth != format!("Server {}", s.config.sfu_token) {
        return Err(Error::MissingAuth);
    }

    Ok(ws.on_upgrade(move |socket| sfu_worker(s, socket)))
}

async fn sfu_worker(s: Arc<ServerState>, mut socket: WebSocket) {
    let mut outbox = s.sushi_sfu.subscribe();
    loop {
        select! {
            msg = outbox.recv() => {
                let stringified = serde_json::to_string(&msg.unwrap()).unwrap();
                if let Err(e) = socket.send(Message::text(stringified)).await {
                    error!("Failed to send message to websocket: {:?}", e);
                    break;
                }
            }
            msg = socket.recv() => {
                if let Some(Ok(Message::Text(text))) = msg {
                    if let Ok(json) = serde_json::from_str::<SfuCommand>(&text) {
                        let s = s.clone();
                        tokio::spawn(async move {
                            let result = match json {
                                #[cfg(feature = "voice")]
                                SfuCommand::VoiceDispatch { user_id, payload } => {
                                    s.broadcast(MessageSync::VoiceDispatch { user_id, payload })
                                }
                                #[cfg(feature = "voice")]
                                SfuCommand::VoiceState {
                                    user_id,
                                    old,
                                    state,
                                } => {
                                    if let Some(v) = &old {
                                        let _ = s
                                            .broadcast_thread(
                                                v.thread_id,
                                                user_id,
                                                MessageSync::VoiceState {
                                                    user_id,
                                                    state: state.clone(),
                                                },
                                            )
                                            .await;
                                    }
                                    if let Some(v) = &state {
                                        let _ = s
                                            .broadcast_thread(
                                                v.thread_id,
                                                user_id,
                                                MessageSync::VoiceState { user_id, state },
                                            )
                                            .await;
                                    }
                                    Ok(())
                                }
                            };
                            if let Err(e) = result {
                                error!("Error processing SFU command: {:?}", e);
                            }
                        });
                    }
                } else {
                    break;
                }
            }
        }
    }
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes!(internal_rpc))
}
