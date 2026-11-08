use common::v1::types::{
    AuditLogEntryType, MessageSync, Permission, emoji::EmojiOwner, oauth::Scope, util::Changes,
    util::Diff,
};

use crate::{prelude::*, util::request::ReqAuth};

#[handler(routes::emoji_create)]
pub async fn create(
    req: Req<routes::emoji_create::Endpoint>,
) -> Result<routes::emoji_create::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();
    body.emoji
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    srv.perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::EmojiManage)
        .check()?;

    let mut auth = ReqAuth::new(&req);
    let emoji = srv
        .emoji
        .create(
            body.room_id,
            &mut auth,
            body.emoji.clone(),
            body.idempotency_key.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::emoji_create::Response { emoji })
}

#[handler(routes::emoji_get)]
pub async fn get(req: Req<routes::emoji_get::Endpoint>) -> Result<routes::emoji_get::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let emoji = srv.emoji.get(body.emoji_id).await.cast_internal()?;

    if let Some(EmojiOwner::Room { room_id }) = emoji.owner {
        srv.perms
            .for_room3(identity.user_id(), room_id)
            .await
            .cast_internal()?
            .ensure_view()?
            .check()?;
    }

    Ok(routes::emoji_get::Response { emoji })
}

#[handler(routes::emoji_delete)]
pub async fn delete(
    req: Req<routes::emoji_delete::Endpoint>,
) -> Result<routes::emoji_delete::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let mut data = globals.begin().await.cast_internal()?;

    srv.perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::EmojiManage)
        .check()?;

    let emoji = data.emoji_get(body.emoji_id).await.cast_internal()?;
    data.emoji_delete(body.emoji_id).await.cast_internal()?;
    data.commit().await.cast_internal()?;

    req.audit_log()
        .push(
            body.room_id,
            AuditLogEntryType::EmojiDelete {
                emoji_id: body.emoji_id,
                changes: Changes::new()
                    .remove("name", &emoji.name)
                    .remove("animated", &emoji.animated)
                    .remove("media_id", &emoji.media_id)
                    .build(),
            },
        )
        .success();

    if let Some(EmojiOwner::Room { room_id }) = emoji.owner {
        globals
            .messaging()
            .broadcast_room(
                room_id,
                MessageSync::EmojiDelete {
                    emoji_id: emoji.id,
                    room_id,
                },
            )
            .await
            .cast_internal()?;
    }

    Ok(routes::emoji_delete::Response {})
}

#[handler(routes::emoji_update)]
pub async fn update(
    req: Req<routes::emoji_update::Endpoint>,
) -> Result<routes::emoji_update::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    body.patch
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let mut data = globals.begin().await.cast_internal()?;

    srv.perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::EmojiManage)
        .check()?;

    let emoji_before = data.emoji_get(body.emoji_id).await.cast_internal()?;
    if !body.patch.changes(&emoji_before) {
        return Ok(routes::emoji_update::Response {
            emoji: emoji_before,
        });
    }

    data.emoji_update(body.emoji_id, body.patch.clone())
        .await
        .cast_internal()?;
    let emoji = data.emoji_get(body.emoji_id).await.cast_internal()?;
    data.commit().await.cast_internal()?;

    req.audit_log()
        .push(
            body.room_id,
            AuditLogEntryType::EmojiUpdate {
                changes: Changes::new()
                    .change("name", &emoji_before.name, &emoji.name)
                    .build(),
            },
        )
        .success();

    if let Some(EmojiOwner::Room { room_id }) = emoji.owner {
        globals
            .messaging()
            .broadcast_room(
                room_id,
                MessageSync::EmojiUpdate {
                    emoji: emoji.clone(),
                },
            )
            .await
            .cast_internal()?;
    }

    Ok(routes::emoji_update::Response { emoji })
}

#[handler(routes::emoji_list)]
pub async fn list(req: Req<routes::emoji_list::Endpoint>) -> Result<routes::emoji_list::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(identity.user_id(), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let emoji = data
        .emoji_list(body.room_id, body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::emoji_list::Response { emoji })
}

#[handler(routes::emoji_search)]
pub async fn search(
    req: Req<routes::emoji_search::Endpoint>,
) -> Result<routes::emoji_search::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let emoji = data
        .emoji_search(user.id, body.search.query.clone(), body.pagination.clone())
        .await
        .cast_internal()?;

    Ok(routes::emoji_search::Response { emoji })
}

#[handler(routes::emoji_lookup)]
pub async fn lookup(
    req: Req<routes::emoji_lookup::Endpoint>,
) -> Result<routes::emoji_lookup::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let mut emoji = data.emoji_get(body.emoji_id).await.cast_internal()?;

    let original_owner = emoji.owner.clone();
    let original_creator_id = emoji.creator_id;

    emoji.creator_id = None;
    emoji.owner = None;

    match original_owner {
        Some(EmojiOwner::Room { room_id }) => {
            srv.perms
                .for_room3(Some(user.id), room_id)
                .await
                .cast_internal()?
                .ensure_view()?
                .check()?;
            if data.room_member_get(room_id, user.id).await.is_ok() {
                emoji.owner = original_owner;
                emoji.creator_id = original_creator_id;
            }
        }
        Some(EmojiOwner::User) => {
            if original_creator_id == Some(user.id) {
                emoji.owner = original_owner;
                emoji.creator_id = original_creator_id;
            }
        }
        None => {}
    }

    Ok(routes::emoji_lookup::Response { emoji })
}

export_routes!(create, get, delete, update, list, search, lookup);
