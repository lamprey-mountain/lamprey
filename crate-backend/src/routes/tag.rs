use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::{extract::State, Json};
use common::v1::types::tag::{Tag, TagCreate, TagPatch};
use common::v1::types::{PaginationQuery, PaginationResponse, RoomId, TagId, ThreadId};
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
    params(("tag_id", description = "Tag id")),
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
    params(("tag_id", description = "Tag id")),
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
    params(("tag_id", description = "Tag id")),
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

/// Tag thread apply (TODO)
///
/// Apply a tag to a thread. For bulk applying tags, consider editing the thread's tags field directly.
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/tag/{tag_id}",
    tags = ["tag"],
    params(
        ("thread_id", description = "Thread id"),
        ("tag_id", description = "Tag id"),
    ),
    responses(
        (status = NO_CONTENT, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn tag_thread_apply(
    Auth(_session): Auth,
    Path((_thread_id, _tag_id)): Path<(ThreadId, TagId)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag thread unapply (TODO)
///
/// Unapply a tag from a thread. For bulk removing tags, consider editing the thread's tags field directly.
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/tag/{tag_id}",
    tags = ["tag"],
    params(
        ("thread_id", description = "Thread id"),
        ("tag_id", description = "Tag id"),
    ),
    responses(
        (status = NO_CONTENT, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn tag_thread_unapply(
    Auth(_session): Auth,
    Path((_thread_id, _tag_id)): Path<(ThreadId, TagId)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag room apply (TODO)
///
/// Apply a tag to a room
#[utoipa::path(
    put,
    path = "/room/{room_id}/tag/{tag_id}",
    tags = ["tag"],
    params(
        ("room_id", description = "Room id"),
        ("tag_id", description = "Tag id"),
    ),
    responses(
        (status = NO_CONTENT, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn tag_room_apply(
    Auth(_session): Auth,
    Path((_room_id, _tag_id)): Path<(RoomId, TagId)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag room unapply (TODO)
///
/// Unapply a tag from a room
#[utoipa::path(
    delete,
    path = "/room/{room_id}/tag/{tag_id}",
    tags = ["tag"],
    params(
        ("room_id", description = "Room id"),
        ("tag_id", description = "Tag id"),
    ),
    responses(
        (status = NO_CONTENT, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn tag_room_unapply(
    Auth(_session): Auth,
    Path((_room_id, _tag_id)): Path<(RoomId, TagId)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag tag apply (TODO)
///
/// Apply a tag to a tag
///
/// If tag a is tagged with tag b then any taggable tagged with tag a is implicitly tagged with tag b
#[utoipa::path(
    put,
    path = "/tag/{tag_id}/tag/{tag_id}",
    tags = ["tag"],
    params(
        ("target_id", description = "Target tag id"),
        ("with_id", description = "Tag id of tag to tag tag with"),
    ),
    responses(
        (status = NO_CONTENT, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn tag_tag_apply(
    Auth(_session): Auth,
    Path((_target_id, _with_id)): Path<(TagId, TagId)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag tag unapply (TODO)
///
/// Unapply a tag from a tag
#[utoipa::path(
    delete,
    path = "/tag/{tag_id}/tag/{tag_id}",
    tags = ["tag"],
    params(
        ("target_id", description = "Target tag id"),
        ("with_id", description = "Tag id of tag to tag tag with"),
    ),
    responses(
        (status = NO_CONTENT, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn tag_tag_unapply(
    Auth(_session): Auth,
    Path((_target_id, _with_id)): Path<(TagId, TagId)>,
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
        .routes(routes!(tag_thread_apply))
        .routes(routes!(tag_thread_unapply))
        .routes(routes!(tag_room_apply))
        .routes(routes!(tag_room_unapply))
        .routes(routes!(tag_tag_apply))
        .routes(routes!(tag_tag_unapply))
}
