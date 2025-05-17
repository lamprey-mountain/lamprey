use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use common::v1::types::voice::VoiceState;
use common::v1::types::{MessageSync, UserId};
use http::HeaderMap;
use serde_json::Value;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::{Error, ServerState};

// TODO: does this count as an implementation detail or should it be moved to common?

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "type")]
enum Command {
    #[cfg(feature = "voice")]
    VoiceDispatch { user_id: UserId, payload: Value },

    #[cfg(feature = "voice")]
    VoiceState {
        user_id: UserId,
        old: Option<VoiceState>,
        state: Option<VoiceState>,
    },
}

/// Internal rpc
#[utoipa::path(
    post,
    path = "/internal/rpc",
    tags = ["internal"],
    responses((status = ACCEPTED, description = "Accepted")),
)]
#[allow(unused)]
async fn internal_rpc(
    headers: HeaderMap,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<Command>,
) -> Result<StatusCode> {
    let auth = headers
        .get("authorization")
        .ok_or(Error::MissingAuth)?
        .to_str()?;
    if auth != "Server verysecrettoken" {
        return Err(Error::MissingAuth);
    }
    match dbg!(json) {
        #[cfg(feature = "voice")]
        Command::VoiceDispatch { user_id, payload } => {
            s.broadcast(MessageSync::VoiceDispatch { user_id, payload })?;
        }
        #[cfg(feature = "voice")]
        Command::VoiceState {
            user_id,
            old,
            state,
        } => {
            // TODO: deduplicate
            if let Some(v) = &old {
                s.broadcast_thread(
                    v.thread_id,
                    user_id,
                    None,
                    MessageSync::VoiceState {
                        user_id,
                        state: state.clone(),
                    },
                )
                .await?;
            }
            if let Some(v) = &state {
                s.broadcast_thread(
                    v.thread_id,
                    user_id,
                    None,
                    MessageSync::VoiceState { user_id, state },
                )
                .await?;
            }
        }
    };
    Ok(StatusCode::ACCEPTED)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes!(internal_rpc))
}
