use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::misc::UserIdReq;
use common::v1::types::reaction::{ReactionKey, ReactionKeyParam};
use common::v1::types::{AuditLogEntryType, MessageSync, Permission};
use http::StatusCode;
use lamprey_macros::handler;

use super::util::Auth;
use crate::error::Result;
use crate::routes2;
use crate::ServerState;
use utoipa_axum::router::OpenApiRouter;

/// Reaction list
///
/// List message reactions for a specific emoji.
#[handler(routes::reaction_list)]
async fn reaction_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::reaction_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;
    let list = data
        .reaction_list(
            req.channel_id,
            req.message_id,
            req.reaction_key,
            req.pagination,
        )
        .await?;
    Ok(Json(list))
}

/// Reaction add
///
/// Add a reaction to a message.
#[handler(routes::reaction_add)]
async fn reaction_add(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::reaction_add::Request,
) -> Result<impl IntoResponse> {
    let user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs_unlocked()
        .needs(Permission::ReactionAdd)
        .check()?;

    if auth.user.id != user_id {
        return Err(ApiError::from_code(ErrorCode::CannotActOnBehalfOfOthers).into());
    }

    let thread = s
        .services()
        .channels
        .get(req.channel_id, Some(auth.user.id))
        .await?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    let data = s.data();
    data.reaction_put(
        user_id,
        req.channel_id,
        req.message_id,
        req.reaction_key.clone(),
    )
    .await?;

    let reaction_key = match req.reaction_key {
        ReactionKeyParam::Text(t) => ReactionKey::Text { content: t },
        ReactionKeyParam::Custom(emoji_id) => {
            let emoji = data.emoji_get(emoji_id).await?;
            ReactionKey::Custom(emoji)
        }
    };

    s.broadcast_channel(
        req.channel_id,
        user_id,
        MessageSync::ReactionCreate {
            channel_id: req.channel_id,
            user_id,
            message_id: req.message_id,
            key: reaction_key,
        },
    )
    .await?;

    Ok(Json(()))
}

/// Reaction remove
///
/// Remove a user's reaction from a message.
#[handler(routes::reaction_remove)]
async fn reaction_remove(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::reaction_remove::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    let mut perms = srv
        .perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs_unlocked();

    if auth.user.id == user_id {
        perms.needs(Permission::ReactionAdd);
    } else {
        perms.needs(Permission::ReactionManage);
    }

    perms.check()?;

    let chan = s
        .services()
        .channels
        .get(req.channel_id, Some(auth.user.id))
        .await?;
    chan.ensure_unarchived()?;
    chan.ensure_unremoved()?;

    let data = s.data();
    data.reaction_delete(
        user_id,
        req.channel_id,
        req.message_id,
        req.reaction_key.clone(),
    )
    .await?;

    let reaction_key_for_sync = match req.reaction_key.clone() {
        ReactionKeyParam::Text(t) => ReactionKey::Text { content: t },
        ReactionKeyParam::Custom(emoji_id) => {
            let emoji = data.emoji_get(emoji_id).await?;
            ReactionKey::Custom(emoji)
        }
    };

    if let Some(room_id) = chan.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::ReactionDeleteUser {
            channel_id: req.channel_id,
            message_id: req.message_id,
            key: req.reaction_key,
            user_id,
        })
        .await?;
    }

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::ReactionDelete {
            channel_id: req.channel_id,
            user_id,
            message_id: req.message_id,
            key: reaction_key_for_sync,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Reaction remove emoji
///
/// Remove all reactions for a specific key/emoji from a message.
#[handler(routes::reaction_remove_emoji)]
async fn reaction_remove_emoji(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::reaction_remove_emoji::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let srv = s.services();

    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs_unlocked()
        .needs(Permission::ReactionManage)
        .check()?;

    let chan = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    chan.ensure_unarchived()?;
    chan.ensure_unremoved()?;

    data.reaction_delete_key(req.channel_id, req.message_id, req.reaction_key.clone())
        .await?;

    let reaction_key_for_sync = match req.reaction_key.clone() {
        ReactionKeyParam::Text(t) => ReactionKey::Text { content: t },
        ReactionKeyParam::Custom(emoji_id) => {
            let emoji = data.emoji_get(emoji_id).await?;
            ReactionKey::Custom(emoji)
        }
    };

    if let Some(room_id) = chan.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::ReactionDeleteKey {
            channel_id: req.channel_id,
            message_id: req.message_id,
            key: req.reaction_key,
        })
        .await?;
    }

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::ReactionDeleteKey {
            channel_id: req.channel_id,
            message_id: req.message_id,
            key: reaction_key_for_sync,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Reaction remove all
///
/// Remove all reactions from a message.
#[handler(routes::reaction_remove_all)]
async fn reaction_remove_all(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::reaction_remove_all::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let data = s.data();
    let srv = s.services();

    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs_unlocked()
        .needs(Permission::ReactionManage)
        .check()?;

    let thread = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    thread.ensure_unarchived()?;
    thread.ensure_unremoved()?;

    data.reaction_delete_all(req.channel_id, req.message_id)
        .await?;

    if let Some(room_id) = thread.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::ReactionDeleteAll {
            channel_id: req.channel_id,
            message_id: req.message_id,
        })
        .await?;
    }

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::ReactionDeleteAll {
            channel_id: req.channel_id,
            message_id: req.message_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(reaction_list))
        .routes(routes2!(reaction_add))
        .routes(routes2!(reaction_remove))
        .routes(routes2!(reaction_remove_emoji))
        .routes(routes2!(reaction_remove_all))
}
