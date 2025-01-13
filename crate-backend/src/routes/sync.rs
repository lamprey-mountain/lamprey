use axum::{extract::State, Json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use crate::error::Result;
use super::util::Auth;

/// Sync init
/// 
/// Open a websocket to start syncing
#[utoipa::path(
    get,
    path = "/sync",
    tags = ["invite"],
    responses(
        (status = UPGRADE_REQUIRED, description = "success"),
    )
)]
pub async fn sync(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        .routes(routes!(sync))
}
