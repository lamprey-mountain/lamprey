use std::sync::Arc;

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::user_config::{
    PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser,
};
use common::v1::types::{ChannelId, MessageSync, RoomId, UserId};
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::Result;
use crate::ServerState;

/// User config global put
#[utoipa::path(
    put,
    path = "/config",
    tags = ["user_config"],
    responses((status = OK, body = PreferencesGlobal, description = "success"))
)]
async fn user_config_global_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<PreferencesGlobal>,
) -> Result<impl IntoResponse> {
    s.data().user_config_set(auth.user.id, &json).await?;
    s.broadcast(MessageSync::UserConfigGlobal {
        user_id: auth.user.id,
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
    responses((status = OK, body = PreferencesRoom, description = "success"))
)]
async fn user_config_room_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(room_id): Path<RoomId>,
    Json(json): Json<PreferencesRoom>,
) -> Result<impl IntoResponse> {
    s.data()
        .user_config_room_set(auth.user.id, room_id, &json)
        .await?;
    s.broadcast(MessageSync::UserConfigRoom {
        user_id: auth.user.id,
        room_id,
        config: json.clone(),
    })?;
    Ok(Json(json))
}

/// User config channel put
#[utoipa::path(
    put,
    path = "/config/thread/{thread_id}",
    params(("thread_id", description = "Thread id")),
    tags = ["user_config"],
    responses((status = OK, body = PreferencesChannel, description = "success"))
)]
async fn user_config_channel_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(channel_id): Path<ChannelId>,
    Json(json): Json<PreferencesChannel>,
) -> Result<impl IntoResponse> {
    s.data()
        .user_config_channel_set(auth.user.id, channel_id, &json)
        .await?;
    s.broadcast(MessageSync::UserConfigChannel {
        user_id: auth.user.id,
        channel_id,
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
    responses((status = OK, body = PreferencesUser, description = "success"))
)]
async fn user_config_user_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(user_id): Path<UserId>,
    Json(json): Json<PreferencesUser>,
) -> Result<impl IntoResponse> {
    s.data()
        .user_config_user_set(auth.user.id, user_id, &json)
        .await?;
    s.broadcast(MessageSync::UserConfigUser {
        user_id: auth.user.id,
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
    responses((status = OK, body = PreferencesGlobal, description = "success"))
)]
async fn user_config_global_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let config = s.data().user_config_get(auth.user.id).await?;
    Ok(Json(config))
}

/// User config room get
#[utoipa::path(
    get,
    path = "/config/room/{room_id}",
    params(("room_id", description = "Room id")),
    tags = ["user_config"],
    responses((status = OK, body = PreferencesRoom, description = "success"))
)]
async fn user_config_room_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(room_id): Path<RoomId>,
) -> Result<impl IntoResponse> {
    let config = s.data().user_config_room_get(auth.user.id, room_id).await?;
    Ok(Json(config))
}

/// User config channel get
#[utoipa::path(
    get,
    path = "/config/channel/{channel_id}",
    params(("channel_id", description = "Channel id")),
    tags = ["user_config"],
    responses((status = OK, body = PreferencesChannel, description = "success"))
)]
async fn user_config_channel_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(channel_id): Path<ChannelId>,
) -> Result<impl IntoResponse> {
    let config = s
        .data()
        .user_config_channel_get(auth.user.id, channel_id)
        .await?;
    Ok(Json(config))
}

/// User config user get
#[utoipa::path(
    get,
    path = "/config/user/{user_id}",
    params(("user_id", description = "User id")),
    tags = ["user_config"],
    responses((status = OK, body = PreferencesUser, description = "success"))
)]
async fn user_config_user_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(user_id): Path<UserId>,
) -> Result<impl IntoResponse> {
    let config = s.data().user_config_user_get(auth.user.id, user_id).await?;
    Ok(Json(config))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    let global_config_put_routes = OpenApiRouter::new()
        .routes(routes!(user_config_global_put))
        .layer(RequestBodyLimitLayer::new(65536)); // 64KiB

    let other_config_put_routes = OpenApiRouter::new()
        .routes(routes!(user_config_room_put))
        .routes(routes!(user_config_channel_put))
        .routes(routes!(user_config_user_put))
        .layer(RequestBodyLimitLayer::new(16384)); // 16KiB

    OpenApiRouter::new()
        .merge(global_config_put_routes)
        .merge(other_config_put_routes)
        .routes(routes!(user_config_global_get))
        .routes(routes!(user_config_room_get))
        .routes(routes!(user_config_channel_get))
        .routes(routes!(user_config_user_get))
}
