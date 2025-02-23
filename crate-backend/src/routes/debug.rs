use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{extract::State, Json};
use serde::Serialize;
use types::{UrlEmbed, UrlEmbedRequest};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use super::util::Auth;
use crate::error::Result;

#[derive(Serialize, ToSchema)]
struct ServerVersion {
    cargo_debug: bool,
    cargo_target: &'static str,
    git_sha: &'static str,
    git_describe: &'static str,
    git_branch: &'static str,
    rustc_semver: &'static str,
    rustc_llvm: &'static str,
    rustc_sha: &'static str,
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
        cargo_debug: env!("VERGEN_CARGO_DEBUG") == "true",
        cargo_target: env!("VERGEN_CARGO_TARGET_TRIPLE"),
        git_sha: env!("VERGEN_GIT_SHA"),
        git_describe: env!("VERGEN_GIT_DESCRIBE"),
        git_branch: env!("VERGEN_GIT_BRANCH"),
        rustc_semver: env!("VERGEN_RUSTC_SEMVER"),
        rustc_llvm: env!("VERGEN_RUSTC_LLVM_VERSION"),
        rustc_sha: env!("VERGEN_RUSTC_COMMIT_HASH"),
        rustc_channel: env!("VERGEN_RUSTC_CHANNEL"),
    }))
}

/// Embed a url
#[utoipa::path(
    post,
    path = "/debug/embed-url",
    tags = ["debug"],
    responses(
        (status = OK, body = UrlEmbed, description = "success"),
    )
)]
pub async fn debug_embed_url(
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<UrlEmbedRequest>,
) -> Result<impl IntoResponse> {
    let embed = s.services().url_embed.generate(user_id, json.url).await?;
    Ok(Json(embed))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(debug_version))
        .routes(routes!(debug_embed_url))
}
