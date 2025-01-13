use axum::{extract::State, Json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use crate::error::Result;
use super::util::Auth;

/// Role create
#[utoipa::path(
    post,
    path = "/rooms/{room_id}/role",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["role"],
    responses(
        (status = CREATED, description = "success"),
    )
)]
pub async fn role_create(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Role update
#[utoipa::path(
    patch,
    path = "/rooms/{room_id}/role/{role_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
        (status = NOT_MODIFIED, description = "success"),
    )
)]
pub async fn role_update(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Role delete
#[utoipa::path(
    delete,
    path = "/rooms/{room_id}/role/{role_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
pub async fn role_delete(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Role get
#[utoipa::path(
    get,
    path = "/rooms/{room_id}/role/{role_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn role_get(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Role list
#[utoipa::path(
    get,
    path = "/rooms/{room_id}/role",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn role_list(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Role list members
#[utoipa::path(
    get,
    path = "/rooms/{room_id}/role/{role_id}/member",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn role_member_list(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Role member apply
#[utoipa::path(
    put,
    path = "/rooms/{room_id}/role/{role_id}/member/{user_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
        ("user_id", description = "User id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn role_member_add(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Role member remove
#[utoipa::path(
    delete,
    path = "/rooms/{room_id}/role/{role_id}/member/{user_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
        ("user_id", description = "User id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn role_member_remove(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        // .routes(routes!(role_create))
        // .routes(routes!(role_update))
        // .routes(routes!(role_delete))
        // .routes(routes!(role_get))
        // .routes(routes!(role_list))
        // .routes(routes!(role_member_list))
        // .routes(routes!(role_member_add))
        // .routes(routes!(role_member_remove))
}
