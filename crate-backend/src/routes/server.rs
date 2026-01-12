use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::types::server::{ServerInfo, ServerModeration};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::ServerState;

use super::util::Auth;

/// Server information
#[utoipa::path(
    get,
    path = "/server/@self",
    tags = ["server"],
    responses(
        (status = OK, body = ServerInfo, description = "Get server info success"),
    )
)]
async fn server_info(State(s): State<Arc<ServerState>>) -> Result<impl IntoResponse> {
    let info = ServerInfo {
        api_url: s.config.api_url.clone(),
        sync_url: s.config.api_url.join("/api/v1/sync").unwrap(),
        html_url: s.config.html_url.clone(),
        cdn_url: s.config.cdn_url.clone(),
    };
    Ok(Json(info))
}

/// Server moderation capabilities
#[utoipa::path(
    get,
    path = "/server/@self/moderation",
    tags = ["server"],
    responses(
        (status = OK, body = ServerModeration, description = "Get server moderation capabilities success"),
    )
)]
async fn server_moderation(
    _auth: Auth, // requires auth
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    // TODO: let server admins configure this
    let moderation = ServerModeration {
        automod_lists: vec![],
        media_scanners: vec![],
    };
    Ok(Json(moderation))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(server_info))
        .routes(routes!(server_moderation))
}
