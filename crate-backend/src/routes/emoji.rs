use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiOwner};
use common::v1::types::UserId;
use common::v1::types::{
    util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, EmojiId, MessageSync,
    PaginationQuery, PaginationResponse, Permission, RoomId,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
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
    tags = [
        "emoji",
        "badge.perm.EmojiAdd",
    ],
    params(
        ("room_id", description = "Room id"),
    ),
    responses(
        (status = CREATED, body = EmojiCustom, description = "new emoji created"),
    )
)]
async fn emoji_create(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Path(room_id): Path<RoomId>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<EmojiCustomCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_room(auth_user.id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::UnusedEmojiAdd)?;

    let data = s.data();
    // FIXME: run this in a transaction
    let (media, _) = data.media_select(json.media_id).await?;
    if !matches!(
        media.source.info,
        common::v1::types::MediaTrackInfo::Image(_)
    ) {
        return Err(Error::BadStatic("media not an image"));
    }
    if !data.media_link_select(json.media_id).await?.is_empty() {
        return Err(Error::BadStatic("media already used"));
    }

    let media_id = json.media_id;
    let emoji = data
        .emoji_create(auth_user.id, room_id, json.clone())
        .await?;
    data.media_link_insert(media_id, *emoji.id, MediaLinkType::CustomEmoji)
        .await?;

    let changes = Changes::new()
        .add("name", &json.name)
        .add("animated", &json.animated)
        .add("media_id", &json.media_id);

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::EmojiCreate {
            changes: changes.build(),
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth_user.id,
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
        (status = OK,  body = EmojiCustom, description = "success"),
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
    tags = [
        "emoji",
        "badge.perm.EmojiAdd",
    ],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn emoji_delete(
    Path((room_id, emoji_id)): Path<(RoomId, EmojiId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let emoji = data.emoji_get(emoji_id).await?;
    let perms = srv.perms.for_room(auth_user.id, room_id).await?;
    perms.ensure_view()?;
    if emoji.creator_id == auth_user.id {
        perms.ensure(Permission::UnusedEmojiAdd)?;
    } else {
        perms.ensure(Permission::EmojiManage)?;
    }
    data.emoji_delete(emoji_id).await?;
    data.media_link_delete_all(*emoji.id).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::EmojiDelete { emoji_id },
    })
    .await?;

    if let EmojiOwner::Room { room_id } = emoji.owner {
        s.broadcast_room(
            room_id,
            auth_user.id,
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
    Auth(user): Auth,
    Query(q): Query<PaginationQuery<EmojiId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_room(user.id, room_id).await?;
    perms.ensure_view()?;
    let emoji = data.emoji_list(room_id, q).await?;
    Ok(Json(emoji))
}

/// Emoji lookup
///
/// Get info about an emoji.
#[utoipa::path(
    get,
    path = "/emoji/{emoji_id}",
    params(("emoji_id", description = "Emoji id")),
    tags = ["emoji"],
    responses(
        (status = OK, body = PaginationResponse<EmojiCustom>, description = "success"),
    )
)]
async fn emoji_lookup(
    Path(emoji_id): Path<EmojiId>,
    Auth(user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let emoji = data.emoji_get(emoji_id).await?;
    match emoji.owner {
        EmojiOwner::Room { room_id } => {
            if data.room_member_get(room_id, user.id).await.is_ok() {
                Ok(Json(EmojiLookup {
                    id: emoji.id,
                    name: emoji.name,
                    creator_id: Some(emoji.creator_id),
                    room_id: Some(room_id),
                    animated: emoji.animated,
                }))
            } else {
                Ok(Json(EmojiLookup {
                    id: emoji.id,
                    name: emoji.name,
                    creator_id: None,
                    room_id: None,
                    animated: emoji.animated,
                }))
            }
        }
        EmojiOwner::User => Ok(Json(EmojiLookup {
            id: emoji.id,
            name: emoji.name,
            creator_id: if user.id == emoji.creator_id {
                Some(user.id)
            } else {
                None
            },
            room_id: None,
            animated: emoji.animated,
        })),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EmojiLookup {
    pub id: EmojiId,
    pub name: String,

    /// not returned unless you're in the room this emoji is in
    pub creator_id: Option<UserId>,

    /// not returned unless you're in the room this emoji is in and owner is a room
    pub room_id: Option<RoomId>,

    pub animated: bool,
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(emoji_create))
        .routes(routes!(emoji_get))
        .routes(routes!(emoji_delete))
        // .routes(routes!(emoji_update))
        .routes(routes!(emoji_list))
        .routes(routes!(emoji_lookup))
}
