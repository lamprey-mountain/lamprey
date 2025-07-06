use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::user_config::UserConfig;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::Result;
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
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<UserConfig>,
) -> Result<impl IntoResponse> {
    s.data().user_config_set(auth_user_id, &json).await?;
    // FIXME: limit max size for config
    s.broadcast(common::v1::types::MessageSync::UserConfig {
        user_id: auth_user_id,
        config: json.clone(),
    })?;
    Ok(Json(json))
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
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let config = s.data().user_config_get(auth_user_id).await?;
    Ok(Json(config))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(user_config_set))
        .routes(routes!(user_config_get))
}
