use axum::{extract::State, Json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use crate::error::Result;
use super::util::Auth;

/// User create
#[utoipa::path(
    post,
    path = "/user",
    tags = ["user"],
    responses(
        (status = CREATED, description = "success"),
    )
)]
pub async fn user_create(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// User list
#[utoipa::path(
    get,
    path = "/user",
    tags = ["user"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn user_list(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// User update
#[utoipa::path(
    patch,
    path = "/user/{user_id}",
    params(
        ("user_id", description = "User id"),
    ),
    tags = ["user"],
    responses(
        (status = CREATED, description = "success"),
    )
)]
pub async fn user_update(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// User delete
#[utoipa::path(
    delete,
    path = "/user/{user_id}",
    params(
        ("user_id", description = "User id"),
    ),
    tags = ["user"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
pub async fn user_delete(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// User get
#[utoipa::path(
    get,
    path = "/user/{user_id}",
    params(
        ("user_id", description = "User id"),
    ),
    tags = ["user"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn user_get(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        .routes(routes!(user_create))
        .routes(routes!(user_list))
        .routes(routes!(user_update))
        .routes(routes!(user_get))
        .routes(routes!(user_delete))
}
