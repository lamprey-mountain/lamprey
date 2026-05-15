use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::MessageSync;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::{routes2, ServerState};

use super::util::Auth;
use crate::error::Result;

/// Interaction create
#[handler(routes::interaction_create)]
async fn interaction_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::interaction_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let inter = srv
        .interactions
        .create(auth.user.id, req.idempotency_key, req.create)
        .await?;

    Ok((StatusCode::CREATED, Json(inter)))
}

/// Interaction respond
#[handler(routes::interaction_respond)]
async fn interaction_respond(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::interaction_respond::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();

    let inter = srv
        .interactions
        .respond(req.interaction_id, req.token, req.response)?;

    Ok((StatusCode::OK, Json(inter)))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(interaction_create))
        .routes(routes2!(interaction_respond))
}
