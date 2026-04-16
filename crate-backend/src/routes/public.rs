use std::sync::Arc;

use axum::extract::State;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::routes::util::Auth3;
use crate::{Error, ServerState};

/// Public rooms list (TODO)
#[utoipa::path(
    get,
    path = "/public/rooms",
    tags = ["public"],
    responses((status = OK, body = (), description = "ok"))
)]
async fn public_rooms(_auth: Auth3, State(_s): State<Arc<ServerState>>) -> Result<()> {
    Err(Error::Unimplemented)
}

/// Public channels list (TODO)
#[utoipa::path(
    get,
    path = "/public/channels",
    tags = ["public"],
    responses((status = OK, body = (), description = "ok"))
)]
async fn public_channels(_auth: Auth3, State(_s): State<Arc<ServerState>>) -> Result<()> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(public_rooms))
        .routes(routes!(public_channels))
}
