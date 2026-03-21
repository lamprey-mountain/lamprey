use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch, EmojiOwner};
use common::v1::types::util::Diff;
use common::v1::types::{
    util::Changes, AuditLogEntryType, EmojiId, MessageSync, Permission, RoomId,
};
use http::StatusCode;
use lamprey_macros::handler;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::error::Result;
use crate::{routes2, ServerState};

use super::util::Auth;

/// Emoji create
#[handler(routes::emoji_create)]
async fn emoji_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::emoji_create::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.emoji.validate()?;

    let srv = s.services();
    let emoji = srv
        .emoji
        .create(req.room_id, &auth, req.emoji, req.idempotency_key)
        .await?;

    Ok(Json(emoji))
}

/// Emoji get
#[handler(routes::emoji_get)]
async fn emoji_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::emoji_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let emoji = data.emoji_get(req.emoji_id).await?;
    Ok(Json(emoji))
}

/// Emoji delete
#[handler(routes::emoji_delete)]
async fn emoji_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::emoji_delete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();
    let emoji = data.emoji_get(req.emoji_id).await?;
    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    perms.ensure(Permission::EmojiManage)?;
    data.emoji_delete(req.emoji_id).await?;

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::EmojiDelete {
        emoji_id: req.emoji_id,
        changes: Changes::new()
            .remove("name", &emoji.name)
            .remove("animated", &emoji.animated)
            .remove("media_id", &emoji.media_id)
            .build(),
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
#[handler(routes::emoji_update)]
async fn emoji_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::emoji_update::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_room(auth.user.id, req.room_id).await?;
    let data = s.data();
    perms.ensure(Permission::EmojiManage)?;

    let emoji_before = data.emoji_get(req.emoji_id).await?;
    if !req.patch.changes(&emoji_before) {
        return Ok(Json(emoji_before));
    }

    data.emoji_update(req.emoji_id, req.patch).await?;
    let emoji = data.emoji_get(req.emoji_id).await?;

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::EmojiUpdate {
        changes: Changes::new()
            .change("name", &emoji_before.name, &emoji.name)
            .build(),
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
#[handler(routes::emoji_list)]
async fn emoji_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::emoji_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();
    let _perms = srv.perms.for_room(auth.user.id, req.room_id).await?;

    let emoji = data.emoji_list(req.room_id, req.pagination).await?;
    Ok(Json(emoji))
}

/// Emoji search
#[handler(routes::emoji_search)]
async fn emoji_search(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::emoji_search::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let emojis = data
        .emoji_search(auth.user.id, req.search.query, req.pagination)
        .await?;
    Ok(Json(emojis))
}

/// Emoji lookup
///
/// Get info about an emoji.
#[handler(routes::emoji_lookup)]
async fn emoji_lookup(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::emoji_lookup::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let mut emoji = data.emoji_get(req.emoji_id).await?;

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

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(emoji_create))
        .routes(routes2!(emoji_get))
        .routes(routes2!(emoji_delete))
        .routes(routes2!(emoji_update))
        .routes(routes2!(emoji_list))
        .routes(routes2!(emoji_lookup))
        .routes(routes2!(emoji_search))
}
