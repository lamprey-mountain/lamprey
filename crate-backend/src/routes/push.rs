use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::types::push::{PushCreate, PushInfo};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::types::PushData;
use crate::{Error, ServerState};

use super::util::Auth;

/// Push register
///
/// register web push for this session
#[utoipa::path(
    post,
    path = "/push",
    request_body = PushCreate,
    tags = ["push"],
    responses((status = OK, body = PushInfo, description = "ok"))
)]
async fn push_register(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<PushCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let data = s.data();

    let push_data = PushData {
        session_id: auth.session.id,
        user_id: auth.user.id,
        endpoint: json.endpoint.clone(),
        key_p256dh: json.keys.p256dh,
        key_auth: json.keys.auth,
    };

    data.push_insert(push_data).await?;

    let config_internal = data.config_get().await?.ok_or_else(|| {
        Error::Internal("internal config not initialized".to_string())
    })?;

    Ok(Json(PushInfo {
        endpoint: json.endpoint,
        server_key: config_internal.vapid_public_key,
    }))
}

/// Push delete
///
/// remove web push for this session
#[utoipa::path(
    delete,
    path = "/push",
    tags = ["push"],
    responses((status = NO_CONTENT, description = "ok"))
)]
async fn push_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    s.data().push_delete(auth.session.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Push get
///
/// get web push subscription for this session/check if web push is enabled for this session
#[utoipa::path(
    get,
    path = "/push",
    tags = ["push"],
    responses((status = OK, body = PushInfo, description = "ok"))
)]
async fn push_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let push = data.push_get(auth.session.id).await?;
    let config_internal = data.config_get().await?.ok_or_else(|| {
        Error::Internal("internal config not initialized".to_string())
    })?;

    Ok(Json(PushInfo {
        endpoint: push.endpoint,
        server_key: config_internal.vapid_public_key,
    }))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(push_register))
        .routes(routes!(push_delete))
        .routes(routes!(push_get))
}
