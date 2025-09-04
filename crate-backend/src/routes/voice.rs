use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use common::v1::types::{ThreadId, UserId};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::{Auth, HeaderReason};

use crate::error::Result;
use crate::{Error, ServerState};

/// Voice member get (TODO)
#[utoipa::path(
    get,
    path = "/voice/{thread_id}/member/{user_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice"],
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_member_get(
    Path((_room_id, _user_id)): Path<(ThreadId, UserId)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Voice member disconnect (TODO)
#[utoipa::path(
    delete,
    path = "/voice/{thread_id}/member/{user_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn voice_member_disconnect(
    Path((_room_id, _user_id)): Path<(ThreadId, UserId)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    HeaderReason(_reason): HeaderReason,
    Json(_json): Json<()>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Voice member move (TODO)
#[utoipa::path(
    post,
    path = "/voice/{thread_id}/member/{user_id}/move",
    params(
        ("thread_id", description = "Thread id"),
        ("user_id", description = "User id"),
    ),
    tags = ["voice"],
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_member_move(
    Path((_room_id, _user_id)): Path<(ThreadId, UserId)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    HeaderReason(_reason): HeaderReason,
    Json(_json): Json<()>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Voice region list (TODO)
#[utoipa::path(
    get,
    path = "/voice/region",
    tags = ["voice"],
    responses(
        (status = OK, body = (), description = "ok"),
    )
)]
async fn voice_region_list(State(_s): State<Arc<ServerState>>) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(voice_member_get))
        .routes(routes!(voice_member_disconnect))
        .routes(routes!(voice_member_move))
        .routes(routes!(voice_region_list))
}
