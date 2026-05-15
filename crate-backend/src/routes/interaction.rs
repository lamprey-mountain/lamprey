use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::MessageSync;
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::{routes2, Error, ServerState};

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
        .respond(req.interaction_id, req.token, req.response)
        .await?;

    Ok((StatusCode::OK, Json(inter)))
}

/// Interaction message create
#[handler(routes::interaction_message_create)]
async fn interaction_message_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::interaction_message_create::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Interaction message get
#[handler(routes::interaction_message_get)]
async fn interaction_message_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::interaction_message_get::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Interaction message edit
#[handler(routes::interaction_message_edit)]
async fn interaction_message_edit(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::interaction_message_edit::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Interaction message delete
#[handler(routes::interaction_message_delete)]
async fn interaction_message_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::interaction_message_delete::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(interaction_create))
        .routes(routes2!(interaction_respond))
        .routes(routes2!(interaction_message_create))
        .routes(routes2!(interaction_message_get))
        .routes(routes2!(interaction_message_edit))
        .routes(routes2!(interaction_message_delete))
}
