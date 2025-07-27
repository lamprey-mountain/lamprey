use std::sync::Arc;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::{Error, ServerState};

/// Public rooms list (TODO)
#[utoipa::path(
    get,
    path = "/public/rooms",
    tags = ["public"],
    responses((status = OK, body = (), description = "ok"))
)]
async fn public_rooms() -> Result<()> {
    Err(Error::Unimplemented)
}

/// Public threads list (TODO)
#[utoipa::path(
    get,
    path = "/public/threads",
    tags = ["public"],
    responses((status = OK, body = (), description = "ok"))
)]
async fn public_threads() -> Result<()> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(public_rooms))
        .routes(routes!(public_threads))
}
