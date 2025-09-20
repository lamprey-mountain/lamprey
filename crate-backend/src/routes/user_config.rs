use std::sync::Arc;

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::user_config::{
    UserConfigGlobal, UserConfigRoom, UserConfigThread, UserConfigUser,
};
use common::v1::types::{RoomId, ThreadId, UserId};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::Result;
use crate::{Error, ServerState};

/// User config set
///
/// Set user config
#[utoipa::path(
    put,
    path = "/user/{user_id}/config",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = OK, body = UserConfigGlobal, description = "success"))
)]
async fn user_config_set(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<UserConfigGlobal>,
) -> Result<impl IntoResponse> {
    s.data().user_config_set(auth_user.id, &json).await?;
    // FIXME: limit max size for config
    s.broadcast(common::v1::types::MessageSync::UserConfig {
        user_id: auth_user.id,
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
    responses((status = OK, body = UserConfigGlobal, description = "success"))
)]
async fn user_config_get(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let config = s.data().user_config_get(auth_user.id).await?;
    Ok(Json(config))
}

/// User config global write (TODO)
#[utoipa::path(
    patch,
    path = "/config",
    params(("thread_id", description = "Thread id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigGlobal, description = "success"))
)]
async fn user_config_global_write(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<UserConfigGlobal>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User config room write (TODO)
#[utoipa::path(
    patch,
    path = "/config/room/{room_id}",
    params(("room_id", description = "Room id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigRoom, description = "success"))
)]
async fn user_config_room_write(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Path(_room_id): Path<RoomId>,
    Json(_json): Json<UserConfigGlobal>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User config thread write (TODO)
#[utoipa::path(
    patch,
    path = "/config/thread/{thread_id}",
    params(("thread_id", description = "Thread id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigThread, description = "success"))
)]
async fn user_config_thread_write(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Path(_thread_id): Path<ThreadId>,
    Json(_json): Json<UserConfigGlobal>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User config user write (TODO)
#[utoipa::path(
    patch,
    path = "/config/user/{user_id}",
    params(("user_id", description = "User id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigUser, description = "success"))
)]
async fn user_config_user_write(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Path(_user_id): Path<UserId>,
    Json(_json): Json<UserConfigGlobal>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User config global read (TODO)
#[utoipa::path(
    get,
    path = "/config",
    params(("thread_id", description = "Thread id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigGlobal, description = "success"))
)]
async fn user_config_global_read(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User config room read (TODO)
#[utoipa::path(
    get,
    path = "/config/room/{room_id}",
    params(("room_id", description = "Room id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigRoom, description = "success"))
)]
async fn user_config_room_read(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Path(_room_id): Path<RoomId>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User config thread read (TODO)
#[utoipa::path(
    get,
    path = "/config/thread/{thread_id}",
    params(("thread_id", description = "Thread id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigThread, description = "success"))
)]
async fn user_config_thread_read(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Path(_thread_id): Path<ThreadId>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User config user read (TODO)
#[utoipa::path(
    get,
    path = "/config/user/{user_id}",
    params(("user_id", description = "User id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigUser, description = "success"))
)]
async fn user_config_user_read(
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    Path(_user_id): Path<UserId>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(user_config_set))
        .routes(routes!(user_config_get))
        .routes(routes!(user_config_global_write))
        .routes(routes!(user_config_room_write))
        .routes(routes!(user_config_thread_write))
        .routes(routes!(user_config_user_write))
        .routes(routes!(user_config_global_read))
        .routes(routes!(user_config_room_read))
        .routes(routes!(user_config_thread_read))
        .routes(routes!(user_config_user_read))
}
