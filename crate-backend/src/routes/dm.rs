use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::{MessageSync, PaginationQuery, PaginationResponse, Room, UserId};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// Dm initialize
///
/// Get or create a direct message thread.
#[utoipa::path(
    post,
    path = "/user/@self/dm/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["dm"],
    responses(
        (status = CREATED, description = "new dm created"),
        (status = OK, description = "already exists"),
    )
)]
async fn dm_init(
    Path(target_user_id): Path<UserId>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let (thread, is_new) = srv.users.init_dm(auth_user_id, target_user_id).await?;
    s.broadcast(MessageSync::ThreadCreate {
        thread: thread.clone(),
    })?;
    if is_new {
        Ok((StatusCode::CREATED, Json(thread)))
    } else {
        Ok((StatusCode::OK, Json(thread)))
    }
}

/// Dm get
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
    Path(target_user_id): Path<UserId>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let Some(thread_id) = data.dm_get(auth_user_id, target_user_id).await? else {
        return Err(Error::NotFound);
    };
    let srv = s.services();
    let thread = srv.threads.get(thread_id, Some(auth_user_id)).await?;
    Ok(Json(thread))
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
) -> Result<()> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(dm_init))
        .routes(routes!(dm_get))
        .routes(routes!(mutual_room_list))
}
