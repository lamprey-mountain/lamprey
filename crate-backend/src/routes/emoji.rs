use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::{extract::State, Json};
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch};
use common::v1::types::{EmojiId, PaginationQuery, PaginationResponse, RoomId};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::{Error, Result};
use crate::ServerState;

/// Emoji create (TODO)
///
/// Create a custom emoji.
#[utoipa::path(
    post,
    path = "/room/{room_id}/emoji",
    tags = ["emoji"],
    params(
        ("room_id", description = "Room id"),
    ),
    responses(
        (status = CREATED, body = EmojiCustom, description = "new emoji created"),
    )
)]
async fn emoji_create(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<EmojiCustomCreate>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Emoji get (TODO)
///
/// Get a custom emoji.
#[utoipa::path(
    get,
    path = "/room/{room_id}/emoji/{emoji_id}",
    params(
        ("room_id", description = "Room id"),
        ("emoji_id", description = "Emoji id"),
    ),
    tags = ["emoji"],
    responses(
        (status = OK,  body=EmojiCustom, description = "success"),
    )
)]
async fn emoji_get(
    Path(_emoji_id): Path<EmojiId>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Emoji delete (TODO)
///
/// Delete a custom emoji.
#[utoipa::path(
    delete,
    path = "/room/{room_id}/emoji/{emoji_id}",
    params(
        ("room_id", description = "Room id"),
        ("emoji_id", description = "Emoji id"),
    ),
    tags = ["emoji"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn emoji_delete(
    Path(_emoji_id): Path<EmojiId>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Emoji update (TODO)
///
/// Edit a custom emoji.
#[utoipa::path(
    patch,
    path = "/room/{room_id}/emoji/{emoji_id}",
    params(
        ("room_id", description = "Room id"),
        ("emoji_id", description = "Emoji id"),
    ),
    tags = ["emoji"],
    responses(
        (status = NOT_MODIFIED, description = "not modified"),
        (status = OK, body = EmojiCustom, description = "success"),
    )
)]
async fn emoji_update(
    Path(_emoji_id): Path<EmojiId>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<EmojiCustomPatch>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Emoji list (TODO)
///
/// List emoji in a room.
#[utoipa::path(
    get,
    path = "/room/{room_id}/emoji",
    params(
        PaginationQuery<EmojiId>,
        ("room_id", description = "Room id"),
    ),
    tags = ["emoji"],
    responses(
        (status = OK, body = PaginationResponse<EmojiCustom>, description = "success"),
    )
)]
async fn emoji_list(
    Path(_room_id): Path<RoomId>,
    Auth(_auth_user_id): Auth,
    Query(_q): Query<PaginationQuery<EmojiId>>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(emoji_create))
        .routes(routes!(emoji_get))
        .routes(routes!(emoji_delete))
        .routes(routes!(emoji_update))
        .routes(routes!(emoji_list))
}
