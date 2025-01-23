use axum::{extract::State, Json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use crate::error::{Error, Result};
use super::util::Auth;

/// Room member list
#[utoipa::path(
    put,
    path = "/rooms/{room_id}/member",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["member"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn room_member_list(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Room member get
#[utoipa::path(
    put,
    path = "/rooms/{room_id}/member/{user_id}",
    params(
        ("room_id", description = "Room id"),
        ("user_id", description = "User id"),
    ),
    tags = ["member"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn room_member_get(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Room member update
#[utoipa::path(
    patch,
    path = "/rooms/{room_id}/member/{user_id}",
    params(
        ("room_id", description = "Room id"),
        ("user_id", description = "User id"),
    ),
    tags = ["member"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn room_member_update(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Room member delete (kick/leave)
#[utoipa::path(
    delete,
    path = "/rooms/{room_id}/member/{user_id}",
    params(
        ("room_id", description = "Room id"),
        ("user_id", description = "User id"),
    ),
    tags = ["member"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn room_member_delete(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        .routes(routes!(room_member_list))
        .routes(routes!(room_member_get))
        .routes(routes!(room_member_update))
        .routes(routes!(room_member_delete))
}
