use axum::{extract::State, Json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use crate::error::Result;
use super::util::Auth;

/// Session create
#[utoipa::path(
    post,
    path = "/session",
    tags = ["session"],
    responses(
        (status = CREATED, description = "success"),
    )
)]
pub async fn session_create(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Session list
#[utoipa::path(
    get,
    path = "/session",
    tags = ["session"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn session_list(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Session update
#[utoipa::path(
    patch,
    path = "/session/{session_id}",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = CREATED, description = "success"),
    )
)]
pub async fn session_update(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Session delete
#[utoipa::path(
    delete,
    path = "/session/{session_id}",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
pub async fn session_delete(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Session get
#[utoipa::path(
    get,
    path = "/session/{session_id}",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn session_get(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        // .routes(routes!(session_create))
        // .routes(routes!(session_list))
        // .routes(routes!(session_update))
        // .routes(routes!(session_get))
        // .routes(routes!(session_delete))
}
