use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch, EmojiOwner};
use common::v1::types::util::Diff;
use common::v1::types::{
    util::Changes, AuditLogEntry, AuditLogEntryId, AuditLogEntryType, EmojiId, MessageSync,
    PaginationQuery, PaginationResponse, Permission, RoomId,
};
use http::StatusCode;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::{Auth2, HeaderReason};
use crate::error::{Error, Result};
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
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    Path(room_id): Path<RoomId>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<EmojiCustomCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::EmojiManage)?;

    let data = s.data();
    let media = data.media_select(json.media_id).await?;
    if !matches!(
        media.inner.source.info,
        common::v1::types::MediaTrackInfo::Image(_)
    ) {
        return Err(Error::BadStatic("media not an image"));
    }

    let emoji = data
        .emoji_create(auth.user.id, room_id, json.clone())
        .await?;

    let changes = Changes::new()
        .add("name", &json.name)
        .add("animated", &json.animated)
        .add("media_id", &json.media_id);

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason: reason.clone(),
        ty: AuditLogEntryType::EmojiCreate {
            changes: changes.build(),
        },
    })
    .await?;

    s.broadcast_room(
        room_id,
        auth.user.id,
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
    _auth: Auth2,
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
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let emoji = data.emoji_get(emoji_id).await?;
    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::EmojiManage)?;
    data.emoji_delete(emoji_id).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason: reason.clone(),
        ty: AuditLogEntryType::EmojiDelete {
            emoji_id,
            changes: Changes::new()
                .remove("name", &emoji.name)
                .remove("animated", &emoji.animated)
                .remove("media_id", &emoji.media_id)
                .build(),
        },
    })
    .await?;

    if let Some(EmojiOwner::Room { room_id }) = emoji.owner {
        s.broadcast_room(
            room_id,
            auth.user.id,
            MessageSync::EmojiDelete {
                emoji_id: emoji.id,
                room_id,
            },
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Emoji update
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
    Path((room_id, emoji_id)): Path<(RoomId, EmojiId)>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(patch): Json<EmojiCustomPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, room_id).await?;
    let data = s.data();
    perms.ensure(Permission::EmojiManage)?;

    let emoji_before = data.emoji_get(emoji_id).await?;
    if patch.changes(&emoji_before) {
        return Ok(Json(emoji_before));
    }

    data.emoji_update(emoji_id, patch).await?;
    let emoji = data.emoji_get(emoji_id).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: Some(auth.session.id),
        reason: reason.clone(),
        ty: AuditLogEntryType::EmojiUpdate {
            changes: Changes::new()
                .change("name", &emoji_before.name, &emoji.name)
                .build(),
        },
    })
    .await?;

    if let Some(EmojiOwner::Room { room_id }) = emoji.owner {
        s.broadcast_room(
            room_id,
            auth.user.id,
            MessageSync::EmojiUpdate {
                emoji: emoji.clone(),
            },
        )
        .await?;
    }

    Ok(Json(emoji))
}

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
    auth: Auth2,
    Query(q): Query<PaginationQuery<EmojiId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let _perms = srv.perms.for_room(auth.user.id, room_id).await?;

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
        (status = OK, body = EmojiCustom, description = "success"),
    )
)]
async fn emoji_lookup(
    Path(emoji_id): Path<EmojiId>,
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let mut emoji = data.emoji_get(emoji_id).await?;

    let original_owner = emoji.owner.clone();
    let original_creator_id = emoji.creator_id;

    emoji.creator_id = None;
    emoji.owner = None;

    match original_owner {
        Some(EmojiOwner::Room { room_id }) => {
            if data.room_member_get(room_id, auth.user.id).await.is_ok() {
                emoji.owner = original_owner;
                emoji.creator_id = original_creator_id;
            }
        }
        Some(EmojiOwner::User) => {
            if original_creator_id == Some(auth.user.id) {
                emoji.owner = original_owner;
                emoji.creator_id = original_creator_id;
            }
        }
        None => {}
    }

    Ok(Json(emoji))
}

// TODO: move to common
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct EmojiSearchQuery {
    pub query: String,
}

/// Emoji search
///
/// Search all emoji the user can see.
#[utoipa::path(
    get,
    path = "/emoji/search",
    params(EmojiSearchQuery, PaginationQuery<EmojiId>),
    tags = ["emoji"],
    responses(
        (status = OK, body = PaginationResponse<EmojiCustom>, description = "success"),
    )
)]
async fn emoji_search(
    auth: Auth2,
    Query(q): Query<EmojiSearchQuery>,
    Query(pagination): Query<PaginationQuery<EmojiId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let emojis = data.emoji_search(auth.user.id, q.query, pagination).await?;
    Ok(Json(emojis))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(emoji_create))
        .routes(routes!(emoji_get))
        .routes(routes!(emoji_delete))
        .routes(routes!(emoji_update))
        .routes(routes!(emoji_list))
        .routes(routes!(emoji_lookup))
        .routes(routes!(emoji_search))
}
