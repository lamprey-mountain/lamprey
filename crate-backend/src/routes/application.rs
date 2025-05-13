use std::sync::Arc;

use axum::extract::State;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// App create
#[utoipa::path(
    post,
    path = "/app",
    tags = ["application"],
    responses((status = CREATED, description = "success"))
)]
async fn app_create(Auth(_auth_user_id): Auth, State(_s): State<Arc<ServerState>>) -> Result<()> {
    Err(Error::Unimplemented)
}

/// App list
#[utoipa::path(
    get,
    path = "/app",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn app_list(Auth(_auth_user_id): Auth, State(_s): State<Arc<ServerState>>) -> Result<()> {
    Err(Error::Unimplemented)
}

/// App get
#[utoipa::path(
    get,
    path = "/app/{app_id}",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn app_get(Auth(_auth_user_id): Auth, State(_s): State<Arc<ServerState>>) -> Result<()> {
    Err(Error::Unimplemented)
}

/// App patch
#[utoipa::path(
    patch,
    path = "/app/{app_id}",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn app_patch(Auth(_auth_user_id): Auth, State(_s): State<Arc<ServerState>>) -> Result<()> {
    Err(Error::Unimplemented)
}

/// App delete
#[utoipa::path(
    delete,
    path = "/app/{app_id}",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn app_delete(Auth(_auth_user_id): Auth, State(_s): State<Arc<ServerState>>) -> Result<()> {
    Err(Error::Unimplemented)
}

/// Puppet create
#[utoipa::path(
    post,
    path = "/app/{app_id}/puppet",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn puppet_create(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<()> {
    Err(Error::Unimplemented)
}

/// Puppet ensure
#[utoipa::path(
    put,
    path = "/app/{app_id}/puppet/{puppet_id}",
    tags = ["application"],
    responses((status = OK, description = "success"))
)]
async fn puppet_ensure(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<()> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(app_create))
        .routes(routes!(app_list))
        .routes(routes!(app_get))
        .routes(routes!(app_patch))
        .routes(routes!(app_delete))
        .routes(routes!(puppet_create))
        .routes(routes!(puppet_ensure))
}
