use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::{extract::State, Json};
use common::v1::types::tag::{Tag, TagCreate, TagPatch};
use common::v1::types::{PaginationQuery, PaginationResponse, TagId};
use serde::Deserialize;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// Tag create (TODO)
#[utoipa::path(
    post,
    path = "/tag",
    tags = ["tag"],
    responses(
        (status = CREATED, body = Tag, description = "success"),
    )
)]
async fn tag_create(
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_body): Json<TagCreate>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag get (TODO)
#[utoipa::path(
    get,
    path = "/tag/{tag_id}",
    tags = ["tag"],
    responses(
        (status = OK, body = Tag, description = "success"),
    )
)]
async fn tag_get(
    Path(_tag_id): Path<TagId>,
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag patch (TODO)
#[utoipa::path(
    patch,
    path = "/tag/{tag_id}",
    tags = ["tag"],
    responses(
        (status = OK, body = Tag, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn tag_patch(
    Path(_tag_id): Path<TagId>,
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_body): Json<TagPatch>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

// TEMP: while scaffolding
#[allow(unused)]
#[derive(Deserialize)]
struct TagDeleteQuery {
    #[serde(default)]
    force: bool,
}

/// Tag delete (TODO)
#[utoipa::path(
    delete,
    path = "/tag/{tag_id}",
    tags = ["tag"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn tag_delete(
    Path(_tag_id): Path<TagId>,
    Query(_query): Query<TagDeleteQuery>,
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag list user (TODO)
///
/// List tags you have access to?
#[utoipa::path(
    get,
    path = "/tag",
    tags = ["tag"],
    params(
        PaginationQuery<TagId>,
    ),
    responses(
        (status = OK, body = PaginationResponse<Tag>, description = "success"),
    )
)]
async fn tag_list_user(
    Auth(_session): Auth,
    Query(_q): Query<PaginationQuery<TagId>>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag list room (TODO)
///
/// List tags in a room?
#[utoipa::path(
    get,
    path = "/room/{room_id}/tag",
    tags = ["tag"],
    params(
        PaginationQuery<TagId>,
        ("room_id", description = "Room id")
    ),
    responses(
        (status = OK, body = PaginationResponse<Tag>, description = "success"),
    )
)]
async fn tag_list_room(
    Auth(_session): Auth,
    Query(_q): Query<PaginationQuery<TagId>>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(tag_create))
        .routes(routes!(tag_get))
        .routes(routes!(tag_patch))
        .routes(routes!(tag_delete))
        .routes(routes!(tag_list_user))
        .routes(routes!(tag_list_room))
}
