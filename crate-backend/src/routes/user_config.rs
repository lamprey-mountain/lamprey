use std::sync::Arc;

use axum::{extract::State, Json};
use types::user_config::UserConfig;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::{Error, Result};
use crate::ServerState;

/// User config set
///
/// Set user config
#[utoipa::path(
    put,
    path = "/user/{user_id}/config",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = OK, body = UserConfig, description = "success"))
)]
async fn user_config_set(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<UserConfig>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// User config get
///
/// Get user config
#[utoipa::path(
    get,
    path = "/user/{user_id}/config",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = OK, body = UserConfig, description = "success"))
)]
async fn user_config_get(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(user_config_set))
        .routes(routes!(user_config_get))
}
