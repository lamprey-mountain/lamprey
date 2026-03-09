use std::sync::Arc;

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::application::Scope;
use common::v1::types::preferences::{
    PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser,
};
use common::v1::types::{ChannelId, MessageSync, RoomId, UserId};
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::Result;
use crate::ServerState;

/// Preferences global put
#[utoipa::path(
    put,
    path = "/preferences",
    tags = ["preferences", "badge.scope.full"],
    request_body = PreferencesGlobal,
    responses((status = OK, body = PreferencesGlobal, description = "success"))
)]
async fn preferences_global_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<PreferencesGlobal>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data().preferences_set(auth.user.id, &json).await?;
    s.services()
        .cache
        .preferences_invalidate(auth.user.id)
        .await;
    s.broadcast(MessageSync::PreferencesGlobal {
        user_id: auth.user.id,
        config: json.clone(),
    })?;
    Ok(Json(json))
}

/// Preferences room put
#[utoipa::path(
    put,
    path = "/preferences/room/{room_id}",
    tags = ["preferences", "badge.scope.full"],
    params(
        ("room_id" = RoomId, Path, description = "Room id"),
    ),
    request_body = PreferencesRoom,
    responses((status = OK, body = PreferencesRoom, description = "success"))
)]
async fn preferences_room_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(room_id): Path<RoomId>,
    Json(json): Json<PreferencesRoom>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data()
        .preferences_room_set(auth.user.id, room_id, &json)
        .await?;
    s.services()
        .cache
        .preferences_room_invalidate(auth.user.id, room_id)
        .await;
    s.broadcast(MessageSync::PreferencesRoom {
        user_id: auth.user.id,
        room_id,
        config: json.clone(),
    })?;
    Ok(Json(json))
}

/// Preferences channel put
#[utoipa::path(
    put,
    path = "/preferences/channel/{channel_id}",
    tags = ["preferences", "badge.scope.full"],
    params(
        ("channel_id" = ChannelId, Path, description = "Channel id"),
    ),
    request_body = PreferencesChannel,
    responses((status = OK, body = PreferencesChannel, description = "success"))
)]
async fn preferences_channel_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(channel_id): Path<ChannelId>,
    Json(json): Json<PreferencesChannel>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data()
        .preferences_channel_set(auth.user.id, channel_id, &json)
        .await?;
    s.services()
        .cache
        .preferences_channel_invalidate(auth.user.id, channel_id)
        .await;
    s.broadcast(MessageSync::PreferencesChannel {
        user_id: auth.user.id,
        channel_id,
        config: json.clone(),
    })?;
    Ok(Json(json))
}

/// Preferences user put
#[utoipa::path(
    put,
    path = "/preferences/user/{user_id}",
    tags = ["preferences", "badge.scope.full"],
    params(
        ("user_id" = UserId, Path, description = "User id"),
    ),
    request_body = PreferencesUser,
    responses((status = OK, body = PreferencesUser, description = "success"))
)]
async fn preferences_user_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(user_id): Path<UserId>,
    Json(json): Json<PreferencesUser>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data()
        .preferences_user_set(auth.user.id, user_id, &json)
        .await?;
    s.services()
        .cache
        .preferences_user_invalidate(auth.user.id, user_id)
        .await;
    s.broadcast(MessageSync::PreferencesUser {
        user_id: auth.user.id,
        target_user_id: user_id,
        config: json.clone(),
    })?;
    Ok(Json(json))
}

/// Preferences global get
#[utoipa::path(
    get,
    path = "/preferences",
    tags = ["preferences", "badge.scope.full"],
    responses((status = OK, body = PreferencesGlobal, description = "success"))
)]
async fn preferences_global_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let config = s.services().cache.preferences_get(auth.user.id).await?;
    Ok(Json(config))
}

/// Preferences room get
#[utoipa::path(
    get,
    path = "/preferences/room/{room_id}",
    params(("room_id", description = "Room id")),
    tags = ["preferences", "badge.scope.full"],
    responses((status = OK, body = PreferencesRoom, description = "success"))
)]
async fn preferences_room_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(room_id): Path<RoomId>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let config = s
        .services()
        .cache
        .preferences_room_get(auth.user.id, room_id)
        .await?;
    Ok(Json(config))
}

/// Preferences channel get
#[utoipa::path(
    get,
    path = "/preferences/channel/{channel_id}",
    params(("channel_id", description = "Channel id")),
    tags = ["preferences", "badge.scope.full"],
    responses((status = OK, body = PreferencesChannel, description = "success"))
)]
async fn preferences_channel_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(channel_id): Path<ChannelId>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let config = s
        .services()
        .cache
        .preferences_channel_get(auth.user.id, channel_id)
        .await?;
    Ok(Json(config))
}

/// Preferences user get
#[utoipa::path(
    get,
    path = "/preferences/user/{user_id}",
    params(("user_id", description = "User id")),
    tags = ["preferences", "badge.scope.full"],
    responses((status = OK, body = PreferencesUser, description = "success"))
)]
async fn preferences_user_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Path(user_id): Path<UserId>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let config = s
        .services()
        .cache
        .preferences_user_get(auth.user.id, user_id)
        .await?;
    Ok(Json(config))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    let global_config_put_routes = OpenApiRouter::new()
        .routes(routes!(preferences_global_put))
        .layer(RequestBodyLimitLayer::new(65536)); // 64KiB

    let other_config_put_routes = OpenApiRouter::new()
        .routes(routes!(preferences_room_put))
        .routes(routes!(preferences_channel_put))
        .routes(routes!(preferences_user_put))
        .layer(RequestBodyLimitLayer::new(16384)); // 16KiB

    OpenApiRouter::new()
        .merge(global_config_put_routes)
        .merge(other_config_put_routes)
        .routes(routes!(preferences_global_get))
        .routes(routes!(preferences_room_get))
        .routes(routes!(preferences_channel_get))
        .routes(routes!(preferences_user_get))
}
