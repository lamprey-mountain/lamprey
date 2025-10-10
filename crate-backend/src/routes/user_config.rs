use std::sync::Arc;

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::user_config::{
    UserConfigGlobal, UserConfigRoom, UserConfigThread, UserConfigUser,
};
use common::v1::types::{MessageSync, RoomId, ThreadId, UserId};
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
    responses((status = OK, body = UserConfigGlobal, description = "success")),
)]
#[deprecated]
async fn user_config_set(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<UserConfigGlobal>,
) -> Result<impl IntoResponse> {
    s.data().user_config_set(auth_user.id, &json).await?;
    // FIXME: limit max size for config
    s.broadcast(MessageSync::UserConfigGlobal {
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
    responses((status = OK, body = UserConfigGlobal, description = "success")),
)]
#[deprecated]
async fn user_config_get(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let config = s.data().user_config_get(auth_user.id).await?;
    Ok(Json(config))
}

/// User config global put
#[utoipa::path(
    put,
    path = "/config",
    tags = ["user_config"],
    responses((status = OK, body = UserConfigGlobal, description = "success"))
)]
async fn user_config_global_put(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<UserConfigGlobal>,
) -> Result<impl IntoResponse> {
    s.data().user_config_set(auth_user.id, &json).await?;
    s.broadcast(MessageSync::UserConfigGlobal {
        user_id: auth_user.id,
        config: json.clone(),
    })?;
    Ok(Json(json))
}

/// User config room put
#[utoipa::path(
    put,
    path = "/config/room/{room_id}",
    params(("room_id", description = "Room id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigRoom, description = "success"))
)]
async fn user_config_room_put(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Path(room_id): Path<RoomId>,
    Json(json): Json<UserConfigRoom>,
) -> Result<impl IntoResponse> {
    s.data()
        .user_config_room_set(auth_user.id, room_id, &json)
        .await?;
    s.broadcast(MessageSync::UserConfigRoom {
        user_id: auth_user.id,
        room_id,
        config: json.clone(),
    })?;
    Ok(Json(json))
}

/// User config thread put
#[utoipa::path(
    put,
    path = "/config/thread/{thread_id}",
    params(("thread_id", description = "Thread id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigThread, description = "success"))
)]
async fn user_config_thread_put(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Path(thread_id): Path<ThreadId>,
    Json(json): Json<UserConfigThread>,
) -> Result<impl IntoResponse> {
    s.data()
        .user_config_thread_set(auth_user.id, thread_id, &json)
        .await?;
    s.broadcast(MessageSync::UserConfigThread {
        user_id: auth_user.id,
        thread_id,
        config: json.clone(),
    })?;
    Ok(Json(json))
}

/// User config user put
#[utoipa::path(
    put,
    path = "/config/user/{user_id}",
    params(("user_id", description = "User id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigUser, description = "success"))
)]
async fn user_config_user_put(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Path(user_id): Path<UserId>,
    Json(json): Json<UserConfigUser>,
) -> Result<impl IntoResponse> {
    s.data()
        .user_config_user_set(auth_user.id, user_id, &json)
        .await?;
    s.broadcast(MessageSync::UserConfigUser {
        user_id: auth_user.id,
        target_user_id: user_id,
        config: json.clone(),
    })?;
    Ok(Json(json))
}

/// User config global get
#[utoipa::path(
    get,
    path = "/config",
    tags = ["user_config"],
    responses((status = OK, body = UserConfigGlobal, description = "success"))
)]
async fn user_config_global_get(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let config = s.data().user_config_get(auth_user.id).await?;
    Ok(Json(config))
}

/// User config room get
#[utoipa::path(
    get,
    path = "/config/room/{room_id}",
    params(("room_id", description = "Room id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigRoom, description = "success"))
)]
async fn user_config_room_get(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Path(room_id): Path<RoomId>,
) -> Result<impl IntoResponse> {
    let config = s.data().user_config_room_get(auth_user.id, room_id).await?;
    Ok(Json(config))
}

/// User config thread get
#[utoipa::path(
    get,
    path = "/config/thread/{thread_id}",
    params(("thread_id", description = "Thread id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigThread, description = "success"))
)]
async fn user_config_thread_get(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Path(thread_id): Path<ThreadId>,
) -> Result<impl IntoResponse> {
    let config = s
        .data()
        .user_config_thread_get(auth_user.id, thread_id)
        .await?;
    Ok(Json(config))
}

/// User config user get
#[utoipa::path(
    get,
    path = "/config/user/{user_id}",
    params(("user_id", description = "User id")),
    tags = ["user_config"],
    responses((status = OK, body = UserConfigUser, description = "success"))
)]
async fn user_config_user_get(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Path(user_id): Path<UserId>,
) -> Result<impl IntoResponse> {
    let config = s.data().user_config_user_get(auth_user.id, user_id).await?;
    Ok(Json(config))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(user_config_set))
        .routes(routes!(user_config_get))
        .routes(routes!(user_config_global_put))
        .routes(routes!(user_config_room_put))
        .routes(routes!(user_config_thread_put))
        .routes(routes!(user_config_user_put))
        .routes(routes!(user_config_global_get))
        .routes(routes!(user_config_room_get))
        .routes(routes!(user_config_thread_get))
        .routes(routes!(user_config_user_get))
}
