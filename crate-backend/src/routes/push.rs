use std::sync::Arc;

use axum::Json;
use common::v1::types::push::{PushCreate, PushInfo};
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
    responses((status = OK, body = PushInfo, description = "ok"))
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
    responses((status = NO_CONTENT, description = "ok"))
)]
async fn push_delete() -> Result<()> {
    Err(Error::Unimplemented)
}

/// Push get (TODO)
///
/// get web push subscription for this session/check if web push is enabled for this session
#[utoipa::path(
    get,
    path = "/push",
    tags = ["push"],
    responses((status = OK, body = PushInfo, description = "ok"))
)]
async fn push_get() -> Result<()> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(push_register))
        .routes(routes!(push_delete))
        .routes(routes!(push_get))
}
