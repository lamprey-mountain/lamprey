use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    MessageId, MessageSync, MessageThreadUpdate, ThreadState, ThreadType, UserId,
};
use http::HeaderMap;
use serde_json::Value;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::{Error, ServerState};

// TODO: does this count as an implementation detail or should it be moved to common?

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "type")]
enum Command {
    VoiceDispatch { user_id: UserId, payload: Value },
}

/// Internal rpc
#[utoipa::path(
    post,
    path = "/internal/rpc",
    tags = ["internal"],
    responses((status = ACCEPTED, description = "Accepted")),
)]
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
        Command::VoiceDispatch { user_id, payload } => {
            s.broadcast(MessageSync::VoiceDispatch { user_id, payload })?;
        }
    };
    Ok(StatusCode::ACCEPTED)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    let router = OpenApiRouter::new();

    #[cfg(feature = "voice")]
    let router = router.routes(routes!(internal_rpc));

    router
}
