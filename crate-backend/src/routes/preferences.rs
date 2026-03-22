use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::preferences::{
    PreferencesChannel, PreferencesGlobal, PreferencesRoom, PreferencesUser,
};
use common::v1::types::{ChannelId, MessageSync, RoomId, UserId};
use lamprey_macros::handler;
use tower_http::limit::RequestBodyLimitLayer;
use utoipa_axum::router::OpenApiRouter;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Preferences global put
#[handler(routes::preferences_global_put)]
async fn preferences_global_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::preferences_global_put::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data()
        .preferences_set(auth.user.id, &req.preferences)
        .await?;
    s.services()
        .cache
        .preferences_invalidate(auth.user.id)
        .await;
    s.broadcast(MessageSync::PreferencesGlobal {
        user_id: auth.user.id,
        config: req.preferences.clone(),
    })?;
    Ok(Json(req.preferences))
}

/// Preferences room put
#[handler(routes::preferences_room_put)]
async fn preferences_room_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::preferences_room_put::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data()
        .preferences_room_set(auth.user.id, req.room_id, &req.preferences)
        .await?;
    s.services()
        .cache
        .preferences_room_invalidate(auth.user.id, req.room_id)
        .await;
    s.broadcast(MessageSync::PreferencesRoom {
        user_id: auth.user.id,
        room_id: req.room_id,
        config: req.preferences.clone(),
    })?;
    Ok(Json(req.preferences))
}

/// Preferences channel put
#[handler(routes::preferences_channel_put)]
async fn preferences_channel_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::preferences_channel_put::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data()
        .preferences_channel_set(auth.user.id, req.channel_id, &req.preferences)
        .await?;
    s.services()
        .cache
        .preferences_channel_invalidate(auth.user.id, req.channel_id)
        .await;
    s.broadcast(MessageSync::PreferencesChannel {
        user_id: auth.user.id,
        channel_id: req.channel_id,
        config: req.preferences.clone(),
    })?;
    Ok(Json(req.preferences))
}

/// Preferences user put
#[handler(routes::preferences_user_put)]
async fn preferences_user_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::preferences_user_put::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data()
        .preferences_user_set(auth.user.id, req.user_id, &req.preferences)
        .await?;
    s.services()
        .cache
        .preferences_user_invalidate(auth.user.id, req.user_id)
        .await;
    s.broadcast(MessageSync::PreferencesUser {
        user_id: auth.user.id,
        target_user_id: req.user_id,
        config: req.preferences.clone(),
    })?;
    Ok(Json(req.preferences))
}

/// Preferences global get
#[handler(routes::preferences_global_get)]
async fn preferences_global_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::preferences_global_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let config = s.services().cache.preferences_get(auth.user.id).await?;
    Ok(Json(config))
}

/// Preferences room get
#[handler(routes::preferences_room_get)]
async fn preferences_room_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::preferences_room_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let config = s
        .services()
        .cache
        .preferences_room_get(auth.user.id, req.room_id)
        .await?;
    Ok(Json(config))
}

/// Preferences channel get
#[handler(routes::preferences_channel_get)]
async fn preferences_channel_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::preferences_channel_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let config = s
        .services()
        .cache
        .preferences_channel_get(auth.user.id, req.channel_id)
        .await?;
    Ok(Json(config))
}

/// Preferences user get
#[handler(routes::preferences_user_get)]
async fn preferences_user_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::preferences_user_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let config = s
        .services()
        .cache
        .preferences_user_get(auth.user.id, req.user_id)
        .await?;
    Ok(Json(config))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    let global_config_put_routes = OpenApiRouter::new()
        .routes(routes2!(preferences_global_put))
        .layer(RequestBodyLimitLayer::new(65536)); // 64KiB

    let other_config_put_routes = OpenApiRouter::new()
        .routes(routes2!(preferences_room_put))
        .routes(routes2!(preferences_channel_put))
        .routes(routes2!(preferences_user_put))
        .layer(RequestBodyLimitLayer::new(16384)); // 16KiB

    OpenApiRouter::new()
        .merge(global_config_put_routes)
        .merge(other_config_put_routes)
        .routes(routes2!(preferences_global_get))
        .routes(routes2!(preferences_room_get))
        .routes(routes2!(preferences_channel_get))
        .routes(routes2!(preferences_user_get))
}
