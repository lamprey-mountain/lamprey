use common::v1::types::reaction::{ReactionKey, ReactionKeyParam};
use common::v1::types::{AuditLogEntryType, MessageSync, Permission, oauth::Scope};
use http::StatusCode;

use crate::prelude::*;

#[handler(routes::reaction_list)]
pub async fn list(
    req: Req<routes::reaction_list::Endpoint>,
) -> Result<routes::reaction_list::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let reactions = data
        .reaction_list(
            body.channel_id,
            body.message_id,
            body.reaction_key.clone(),
            body.pagination.clone(),
        )
        .await
        .cast_internal()?;

    Ok(routes::reaction_list::Response { reactions })
}

#[handler(routes::reaction_add)]
pub async fn add(
    req: Req<routes::reaction_add::Endpoint>,
) -> Result<routes::reaction_add::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let user_id = body.user_id.clone().unwrap_or(user.id);

    srv.perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs_unlocked()
        .needs(Permission::ReactionAdd)
        .check()?;

    if user.id != user_id {
        return Err(ApiError::from_code(ErrorCode::CannotActOnBehalfOfOthers).into());
    }

    let thread = srv
        .channels
        .get(body.channel_id, Some(user.id))
        .await
        .cast_internal()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.reaction_put(
        user_id,
        body.channel_id,
        body.message_id,
        body.reaction_key.clone(),
    )
    .await
    .cast_internal()?;

    let reaction_key = match body.reaction_key.clone() {
        ReactionKeyParam::Text(t) => ReactionKey::Text { content: t },
        ReactionKeyParam::Custom(emoji_id) => {
            let emoji = data.emoji_get(emoji_id).await.cast_internal()?;
            ReactionKey::Custom(emoji)
        }
    };
    data.commit().await.cast_internal()?;

    globals
        .messaging()
        .broadcast_channel(
            body.channel_id,
            MessageSync::ReactionCreate {
                channel_id: body.channel_id,
                user_id,
                message_id: body.message_id,
                key: reaction_key,
                room_id: thread.room_id,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::reaction_add::Response {
        status: StatusCode::OK,
    })
}

#[handler(routes::reaction_remove)]
pub async fn remove(
    req: Req<routes::reaction_remove::Endpoint>,
) -> Result<routes::reaction_remove::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let user_id = body.user_id.clone().unwrap_or(user.id);

    let mut perms = srv
        .perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.needs_unlocked();

    if user.id == user_id {
        perms.needs(Permission::ReactionAdd);
    } else {
        perms.needs(Permission::ReactionManage);
    }
    perms.check()?;

    let chan = srv
        .channels
        .get(body.channel_id, Some(user.id))
        .await
        .cast_internal()?;
    chan.ensure_unarchived()?;
    chan.ensure_unremoved()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.reaction_delete(
        user_id,
        body.channel_id,
        body.message_id,
        body.reaction_key.clone(),
    )
    .await
    .cast_internal()?;

    let reaction_key = match body.reaction_key.clone() {
        ReactionKeyParam::Text(t) => ReactionKey::Text { content: t },
        ReactionKeyParam::Custom(emoji_id) => {
            let emoji = data.emoji_get(emoji_id).await.cast_internal()?;
            ReactionKey::Custom(emoji)
        }
    };
    data.commit().await.cast_internal()?;

    if let Some(room_id) = chan.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::ReactionDeleteUser {
                    channel_id: body.channel_id,
                    message_id: body.message_id,
                    key: body.reaction_key.clone(),
                    user_id,
                },
            )
            .success();
    }

    globals
        .messaging()
        .broadcast_channel(
            body.channel_id,
            MessageSync::ReactionDelete {
                channel_id: body.channel_id,
                user_id,
                message_id: body.message_id,
                key: reaction_key,
                room_id: chan.room_id,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::reaction_remove::Response {})
}

#[handler(routes::reaction_remove_emoji)]
pub async fn remove_emoji(
    req: Req<routes::reaction_remove_emoji::Endpoint>,
) -> Result<routes::reaction_remove_emoji::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs_unlocked()
        .needs(Permission::ReactionManage)
        .check()?;

    let chan = srv
        .channels
        .get(body.channel_id, Some(user.id))
        .await
        .cast_internal()?;
    chan.ensure_unarchived()?;
    chan.ensure_unremoved()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.reaction_delete_key(body.channel_id, body.message_id, body.reaction_key.clone())
        .await
        .cast_internal()?;

    let reaction_key = match body.reaction_key.clone() {
        ReactionKeyParam::Text(t) => ReactionKey::Text { content: t },
        ReactionKeyParam::Custom(emoji_id) => {
            let emoji = data.emoji_get(emoji_id).await.cast_internal()?;
            ReactionKey::Custom(emoji)
        }
    };
    data.commit().await.cast_internal()?;

    if let Some(room_id) = chan.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::ReactionDeleteKey {
                    channel_id: body.channel_id,
                    message_id: body.message_id,
                    key: body.reaction_key.clone(),
                },
            )
            .success();
    }

    globals
        .messaging()
        .broadcast_channel(
            body.channel_id,
            MessageSync::ReactionDeleteKey {
                channel_id: body.channel_id,
                message_id: body.message_id,
                key: reaction_key,
                room_id: chan.room_id,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::reaction_remove_emoji::Response {})
}

#[handler(routes::reaction_remove_all)]
pub async fn remove_all(
    req: Req<routes::reaction_remove_all::Endpoint>,
) -> Result<routes::reaction_remove_all::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_channel3(Some(user.id), body.channel_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs_unlocked()
        .needs(Permission::ReactionManage)
        .check()?;

    let thread = srv
        .channels
        .get(body.channel_id, Some(user.id))
        .await
        .cast_internal()?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.reaction_delete_all(body.channel_id, body.message_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    if let Some(room_id) = thread.room_id {
        req.audit_log()
            .push(
                room_id,
                AuditLogEntryType::ReactionDeleteAll {
                    channel_id: body.channel_id,
                    message_id: body.message_id,
                },
            )
            .success();
    }

    globals
        .messaging()
        .broadcast_channel(
            body.channel_id,
            MessageSync::ReactionDeleteAll {
                channel_id: body.channel_id,
                message_id: body.message_id,
                room_id: thread.room_id,
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::reaction_remove_all::Response {})
}

export_routes!(list, add, remove, remove_emoji, remove_all);
