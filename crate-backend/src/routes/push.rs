use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::push::PushInfo;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::types::PushData;
use crate::{routes2, Error, ServerState};

use super::util::Auth;
use crate::error::Result;

/// Push register
#[handler(routes::push_register)]
async fn push_register(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::push_register::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let data = s.data();

    let push_data = PushData {
        session_id: auth.session.id,
        user_id: auth.user.id,
        endpoint: req.push.endpoint.clone(),
        key_p256dh: req.push.keys.p256dh,
        key_auth: req.push.keys.auth,
    };

    data.push_insert(push_data).await?;

    let config_internal = data
        .config_get()
        .await?
        .ok_or_else(|| Error::Internal("internal config not initialized".to_string()))?;

    Ok(Json(PushInfo {
        endpoint: req.push.endpoint,
        server_key: config_internal.vapid_public_key,
    }))
}

/// Push delete
#[handler(routes::push_delete)]
async fn push_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::push_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data().push_delete(auth.session.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Push get
#[handler(routes::push_get)]
async fn push_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::push_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let push = data.push_get(auth.session.id).await?;
    let config_internal = data
        .config_get()
        .await?
        .ok_or_else(|| Error::Internal("internal config not initialized".to_string()))?;

    Ok(Json(PushInfo {
        endpoint: push.endpoint,
        server_key: config_internal.vapid_public_key,
    }))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(push_register))
        .routes(routes2!(push_delete))
        .routes(routes2!(push_get))
}
