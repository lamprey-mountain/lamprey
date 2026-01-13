use std::sync::Arc;

use axum::Json;
use common::v1::types::push::PushCreate;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::{Error, ServerState};

/// Push register (TODO)
///
/// register web push for this session
#[utoipa::path(
    post,
    path = "/push",
    request_body = PushCreate,
    tags = ["push"],
    responses((status = OK, body = (), description = "ok"))
)]
async fn push_register(Json(_json): Json<PushCreate>) -> Result<()> {
    Err(Error::Unimplemented)
}

/// Push delete (TODO)
///
/// remove web push for this session
#[utoipa::path(
    delete,
    path = "/push",
    tags = ["push"],
    responses((status = OK, body = (), description = "ok"))
)]
async fn push_delete() -> Result<()> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(push_register))
        .routes(routes!(push_delete))
}
