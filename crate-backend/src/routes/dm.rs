use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::{extract::State, Json};
use types::{PaginationQuery, PaginationResponse, Room, UserId};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// Dm initialize (TODO)
///
/// Get or create a direct message room.
#[utoipa::path(
    put,
    path = "/user/@self/dm/{target_id}",
    params(
        ("target_id", description = "Target user's id"),
    ),
    tags = ["dm"],
    responses(
        (status = CREATED, description = "new dm created"),
        (status = OK, description = "already exists"),
    )
)]
async fn dm_init(
    Path(_target_user_id): Path<UserId>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<Room>> {
    Err(Error::Unimplemented)
}

/// Dm get (TODO)
///
/// Get a direct message room.
#[utoipa::path(
    get,
    path = "/user/@self/dm/{target_id}",
    params(
        ("target_id", description = "Target user's id"),
    ),
    tags = ["dm"],
    responses(
        (status = OK, description = "success"),
    )
)]
async fn dm_get(
    Path(_target_user_id): Path<UserId>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<Room>> {
    Err(Error::Unimplemented)
}

/// Mutual rooms list (TODO)
///
/// List rooms both you and the target are in. Calling it on yourself lists
/// rooms you're in.
#[utoipa::path(
    get,
    path = "/user/{user_id}/room",
    params(
        PaginationQuery<RoomId>,
        ("user_id", description = "user id"),
    ),
    tags = ["dm"],
    responses(
        (status = OK, body = PaginationResponse<Room>, description = "success"),
    )
)]
async fn mutual_room_list(
    Path(_target_user_id): Path<UserIdReq>,
    Auth(_auth_user_id): Auth,
    Query(_q): Query<PaginationQuery<UserId>>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<Room>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(dm_init))
        .routes(routes!(dm_get))
        .routes(routes!(mutual_room_list))
}
