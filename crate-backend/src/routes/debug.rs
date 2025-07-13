use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{extract::State, http::StatusCode, Json};
use common::v1::types::EmbedRequest;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use super::util::Auth;
use crate::error::Result;

#[derive(Serialize, ToSchema)]
struct ServerVersion {
    debug: bool,
    target: &'static str,
    rev: &'static str,
    rustc_semver: &'static str,
    rustc_llvm: &'static str,
    rustc_rev: &'static str,
    rustc_channel: &'static str,
}

/// Get server version
#[utoipa::path(
    get,
    path = "/debug/version",
    tags = ["debug"],
    responses(
        (status = OK, body = ServerVersion, description = "success"),
    )
)]
pub async fn debug_version() -> Result<impl IntoResponse> {
    Ok(Json(ServerVersion {
        debug: env!("VERGEN_CARGO_DEBUG") == "true",
        target: env!("VERGEN_CARGO_TARGET_TRIPLE"),
        rev: env!("VERGEN_GIT_SHA"),
        rustc_semver: env!("VERGEN_RUSTC_SEMVER"),
        rustc_llvm: env!("VERGEN_RUSTC_LLVM_VERSION"),
        rustc_rev: env!("VERGEN_RUSTC_COMMIT_HASH"),
        rustc_channel: env!("VERGEN_RUSTC_CHANNEL"),
    }))
}

/// Embed a url
#[utoipa::path(
    post,
    path = "/debug/embed-url",
    tags = ["debug"],
    responses(
        (status = ACCEPTED, description = "success"),
    )
)]
pub async fn debug_embed_url(
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<EmbedRequest>,
) -> Result<impl IntoResponse> {
    s.services().embed.queue(None, user_id, json.url).await?;
    Ok(StatusCode::ACCEPTED)
}

/// Trigger a panic
#[utoipa::path(
    get,
    path = "/debug/panic",
    tags = ["debug"],
    responses((status = INTERNAL_SERVER_ERROR, description = "success")),
)]
pub async fn debug_panic() {
    panic!("whoops!")
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(debug_version))
        .routes(routes!(debug_embed_url))
        .routes(routes!(debug_panic))
}
