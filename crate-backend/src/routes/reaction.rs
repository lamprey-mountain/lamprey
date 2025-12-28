use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::misc::UserIdReq;
use common::v1::types::reaction::{ReactionKey, ReactionKeyParam, ReactionListItem};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, ChannelId, MessageId, MessageSync,
    PaginationQuery, PaginationResponse, Permission, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::{Auth2, HeaderReason};
use crate::error::Result;
use crate::{Error, ServerState};

/// Reaction list
///
/// List message reactions for a specific emoji.
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}",
    params(
        PaginationQuery<UserId>,
        ("channel_id", description = "channel id"),
        ("message_id", description = "Message id"),
        ("reaction_key", description = "Reaction key"),
    ),
    tags = ["reaction"],
    responses(
        (status = OK, body = PaginationResponse<ReactionListItem>, description = "success"),
    )
)]
async fn reaction_list(
    Path((channel_id, message_id, reaction_key)): Path<(ChannelId, MessageId, ReactionKeyParam)>,
    auth: Auth2,
    Query(q): Query<PaginationQuery<UserId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let list = data
        .reaction_list(channel_id, message_id, reaction_key, q)
        .await?;
    Ok(Json(list))
}

/// Reaction add
///
/// Add a reaction to a message.
#[utoipa::path(
    put,
    path = "/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}/{user_id}",
    params(
        ("channel_id", description = "channel id"),
        ("message_id", description = "Message id"),
        ("reaction_key", description = "Reaction key"),
        ("user_id", description = "User id"),
    ),
    tags = ["reaction", "badge.perm.ReactionAdd"],
    responses(
        (status = CREATED, description = "new reaction created"),
        (status = OK, description = "already exists"),
    )
)]
async fn reaction_add(
    Path((channel_id, message_id, reaction_key, user_id)): Path<(
        ChannelId,
        MessageId,
        ReactionKeyParam,
        UserIdReq,
    )>,
    auth: Auth2,
    HeaderReason(_reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::ReactionAdd)?;

    if auth.user.id != user_id {
        return Err(Error::BadStatic("cannot act on behalf of other users"));
    }

    let thread = s
        .services()
        .channels
        .get(channel_id, Some(auth.user.id))
        .await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
    }

    let data = s.data();
    data.reaction_put(user_id, channel_id, message_id, reaction_key.clone())
        .await?;

    let reaction_key = match reaction_key {
        ReactionKeyParam::Text(t) => ReactionKey::Text { content: t },
        ReactionKeyParam::Custom(emoji_id) => {
            let emoji = data.emoji_get(emoji_id).await?;
            ReactionKey::Custom(emoji)
        }
    };

    s.broadcast_channel(
        channel_id,
        user_id,
        MessageSync::ReactionCreate {
            channel_id,
            user_id,
            message_id,
            key: reaction_key,
        },
    )
    .await?;

    Ok(StatusCode::OK)
}

/// Reaction remove
///
/// Remove a user's reaction from a message.
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}/{user_id}",
    params(
        ("channel_id", description = "channel id"),
        ("message_id", description = "Message id"),
        ("reaction_key", description = "Reaction key"),
        ("user_id", description = "User id"),
    ),
    tags = ["reaction", "badge.perm.ReactionPurge"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn reaction_remove(
    Path((channel_id, message_id, reaction_key, user_id)): Path<(
        ChannelId,
        MessageId,
        ReactionKeyParam,
        UserIdReq,
    )>,
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    if auth.user.id == user_id {
        perms.ensure(Permission::ReactionAdd)?;
    } else {
        perms.ensure(Permission::ReactionPurge)?;
    }

    let chan = s
        .services()
        .channels
        .get(channel_id, Some(auth.user.id))
        .await?;
    if chan.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if chan.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if chan.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
    }

    let data = s.data();
    data.reaction_delete(user_id, channel_id, message_id, reaction_key.clone())
        .await?;

    let reaction_key_for_sync = match reaction_key.clone() {
        ReactionKeyParam::Text(t) => ReactionKey::Text { content: t },
        ReactionKeyParam::Custom(emoji_id) => {
            let emoji = data.emoji_get(emoji_id).await?;
            ReactionKey::Custom(emoji)
        }
    };

    if let Some(room_id) = chan.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason,
            ty: AuditLogEntryType::ReactionDeleteUser {
                channel_id,
                message_id,
                key: reaction_key,
                user_id,
            },
        })
        .await?;
    }

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::ReactionDelete {
            channel_id,
            user_id,
            message_id,
            key: reaction_key_for_sync,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Reaction remove key
///
/// Remove all reactions for a specific key/emoji from a message.
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/message/{message_id}/reaction/{reaction_key}",
    params(
        ("channel_id", description = "channel id"),
        ("message_id", description = "Message id"),
        ("reaction_key", description = "Reaction key"),
    ),
    tags = ["reaction", "badge.perm.ReactionPurge"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn reaction_remove_key(
    Path((channel_id, message_id, reaction_key)): Path<(ChannelId, MessageId, ReactionKeyParam)>,
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::ReactionPurge)?;

    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if chan.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if chan.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if chan.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
    }

    data.reaction_delete_key(channel_id, message_id, reaction_key.clone())
        .await?;

    let reaction_key_for_sync = match reaction_key.clone() {
        ReactionKeyParam::Text(t) => ReactionKey::Text { content: t },
        ReactionKeyParam::Custom(emoji_id) => {
            let emoji = data.emoji_get(emoji_id).await?;
            ReactionKey::Custom(emoji)
        }
    };

    if let Some(room_id) = chan.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason,
            ty: AuditLogEntryType::ReactionDeleteKey {
                channel_id,
                message_id,
                key: reaction_key,
            },
        })
        .await?;
    }

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::ReactionDeleteKey {
            channel_id,
            message_id,
            key: reaction_key_for_sync,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Reaction remove all
///
/// Remove all reactions from a message.
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/message/{message_id}/reaction",
    params(
        ("channel_id", description = "channel id"),
        ("message_id", description = "Message id"),
    ),
    tags = ["reaction", "badge.perm.ReactionPurge"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn reaction_remove_all(
    Path((channel_id, message_id)): Path<(ChannelId, MessageId)>,
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::ReactionPurge)?;

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
    }

    data.reaction_delete_all(channel_id, message_id).await?;

    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason: reason.clone(),
            ty: AuditLogEntryType::ReactionDeleteAll {
                channel_id,
                message_id,
            },
        })
        .await?;
    }

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::ReactionDeleteAll {
            channel_id,
            message_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(reaction_list))
        .routes(routes!(reaction_add))
        .routes(routes!(reaction_remove))
        .routes(routes!(reaction_remove_key))
        .routes(routes!(reaction_remove_all))
}
