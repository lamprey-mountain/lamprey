use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiOwner};
use common::v1::types::{
    EmojiId, MessageSync, PaginationQuery, PaginationResponse, Permission, RoomId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::{Auth, HeaderReason};
use crate::error::{Error, Result};
use crate::types::MediaLinkType;
use crate::ServerState;

/// Emoji create
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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Path(room_id): Path<RoomId>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<EmojiCustomCreate>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::EmojiAdd)?;

    let data = s.data();
    // FIXME: run this in a transaction
    let (media, _) = data.media_select(json.media_id).await?;
    if !matches!(
        media.source.info,
        common::v1::types::MediaTrackInfo::Image(_)
    ) {
        return Err(Error::BadStatic("media not an image"));
    }
    match media.source.size {
        common::v1::types::MediaSize::Bytes(size) => {
            if size > 1024 * 256 {
                return Err(Error::BadStatic(
                    "media is too big (max file size is 256KiB)",
                ));
            }
        }
        common::v1::types::MediaSize::BytesPerSecond(_) => todo!(),
    }
    if !data.media_link_select(json.media_id).await?.is_empty() {
        return Err(Error::BadStatic("media already used"));
    }

    let media_id = json.media_id;
    let emoji = data.emoji_create(user_id, room_id, json).await?;
    data.media_link_insert(media_id, *emoji.id, MediaLinkType::CustomEmoji)
        .await?;
    s.broadcast_room(
        room_id,
        user_id,
        reason,
        MessageSync::EmojiCreate {
            emoji: emoji.clone(),
        },
    )
    .await?;
    Ok(Json(emoji))
}

/// Emoji get
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
    Path((_room_id, emoji_id)): Path<(RoomId, EmojiId)>,
    Auth(_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let emoji = data.emoji_get(emoji_id).await?;
    Ok(Json(emoji))
}

/// Emoji delete
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
    Path((room_id, emoji_id)): Path<(RoomId, EmojiId)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let emoji = data.emoji_get(emoji_id).await?;
    let perms = srv.perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    if emoji.creator_id == user_id {
        perms.ensure(Permission::EmojiAdd)?;
    } else {
        perms.ensure(Permission::EmojiManage)?;
    }
    data.emoji_delete(emoji_id).await?;
    if let EmojiOwner::Room { room_id } = emoji.owner {
        s.broadcast_room(
            room_id,
            user_id,
            reason,
            MessageSync::EmojiDelete {
                emoji_id: emoji.id,
                room_id,
            },
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

// /// Emoji update (TODO(#311))
// ///
// /// Edit a custom emoji.
// #[utoipa::path(
//     patch,
//     path = "/room/{room_id}/emoji/{emoji_id}",
//     params(
//         ("room_id", description = "Room id"),
//         ("emoji_id", description = "Emoji id"),
//     ),
//     tags = ["emoji"],
//     responses(
//         (status = NOT_MODIFIED, description = "not modified"),
//         (status = OK, body = EmojiCustom, description = "success"),
//     )
// )]
// async fn emoji_update(
//     Path(_emoji_id): Path<EmojiId>,
//     Auth(_auth_user_id): Auth,
//     State(_s): State<Arc<ServerState>>,
//     Json(_json): Json<EmojiCustomPatch>,
// ) -> Result<Json<()>> {
//     Err(Error::Unimplemented)
// }

/// Emoji list
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
    Path(room_id): Path<RoomId>,
    Auth(user_id): Auth,
    Query(q): Query<PaginationQuery<EmojiId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    let emoji = data.emoji_list(room_id, q).await?;
    Ok(Json(emoji))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(emoji_create))
        .routes(routes!(emoji_get))
        .routes(routes!(emoji_delete))
        // .routes(routes!(emoji_update))
        .routes(routes!(emoji_list))
}
