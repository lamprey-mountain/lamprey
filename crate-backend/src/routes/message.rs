use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, ContextQuery, ContextResponse,
    MessageMigrate, MessageModerate, MessagePin, MessageType, PinsReorder, RepliesQuery,
    ThreadMemberPut, ThreadMembership,
};
use common::v2::types::message::Message;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Error,
    types::{
        ChannelId, DbMessageCreate, MessageCreate, MessageId, MessagePatch, MessageSync,
        MessageVerId, PaginationQuery, PaginationResponse, Permission,
    },
    ServerState,
};

use super::util::{Auth, HeaderIdempotencyKey, HeaderReason};
use crate::error::Result;

/// Message create
///
/// Send a message to a channel
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/message",
    params(("channel_id", description = "Channel id")),
    tags = [
        "message",
        "badge.perm.MessageCreate",
        "badge.perm-opt.MessageAttachments",
        "badge.perm-opt.MessageEmbeds",
        "badge.perm-opt.MemberBridge",
    ],
    responses(
        (status = CREATED, body = Message, description = "Create message success"),
    )
)]
async fn message_create(
    Path((channel_id,)): Path<(ChannelId,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    HeaderIdempotencyKey(nonce): HeaderIdempotencyKey,
    Json(json): Json<MessageCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let chan = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !chan.ty.has_text() {
        return Err(Error::BadStatic("channel doesnt have text"));
    }

    let message = srv
        .messages
        .create(channel_id, auth.user.id, reason, nonce, json)
        .await?;

    Ok((StatusCode::CREATED, Json(message)))
}

/// Message context
///
/// More efficient than calling List messages twice
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/context/{message_id}",
    params(
        ContextQuery,
        ("channel_id", description = "Channel id"),
        ("message_id", description = "Message id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = ContextResponse, description = "List thread messages success"),
    )
)]
async fn message_context(
    Path((channel_id, message_id)): Path<(ChannelId, MessageId)>,
    Query(q): Query<ContextQuery>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let res = srv
        .messages
        .list_context(channel_id, message_id, auth.user.id, q)
        .await?;

    Ok(Json(res))
}

/// Messages list
///
/// Paginate messages in a thread
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/message",
    params(PaginationQuery<MessageId>, ("channel_id", description = "Channel id")),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List thread messages success"),
    )
)]
async fn message_list(
    Path((channel_id,)): Path<(ChannelId,)>,
    Query(q): Query<PaginationQuery<MessageId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let res = srv.messages.list(channel_id, auth.user.id, q).await?;
    Ok(Json(res))
}

/// Message get
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/message/{message_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, body = Message, description = "List thread messages success"),
    )
)]
async fn message_get(
    Path((channel_id, message_id)): Path<(ChannelId, MessageId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let message = srv
        .messages
        .get(channel_id, message_id, auth.user.id)
        .await?;
    Ok(Json(message))
}

/// Message edit
#[utoipa::path(
    patch,
    path = "/channel/{channel_id}/message/{message_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, body = Message, description = "edit message success"),
        (status = NOT_MODIFIED, description = "no change"),
    )
)]
async fn message_edit(
    Path((channel_id, message_id)): Path<(ChannelId, MessageId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<MessagePatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !thread.ty.has_text() {
        return Err(Error::BadStatic("channel doesnt have text"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    if thread.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
    }

    let (_status, message) = srv
        .messages
        .edit(channel_id, message_id, auth.user.id, reason, json)
        .await?;
    Ok((StatusCode::OK, Json(message)))
}

/// Message delete (TEMP?)
///
/// Note that this endpoint allows deleting your own messages, while message
/// moderate always requires the full permission
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/message/{message_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("message_id", description = "Message id")
    ),
    tags = [
        "message",
        "badge.perm-opt.MessageDelete",
        "badge.room-mfa-opt",
    ],
    responses(
        (status = NO_CONTENT, description = "delete message success"),
    )
)]
async fn message_delete(
    Path((channel_id, message_id)): Path<(ChannelId, MessageId)>,
    auth: Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    // FIXME: allow deleting your own messages without mfa
    if let Some(room_id) = thread.room_id {
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(Error::BadStatic("mfa required for this action"));
            }
        }
    }

    let mut perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let message = data
        .message_get(channel_id, message_id, auth.user.id)
        .await?;
    if !message.latest_version.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete that message"));
    }
    if message.author_id == auth.user.id {
        perms.add(Permission::MessageDelete);
    }
    perms.ensure(Permission::MessageDelete)?;
    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !thread.ty.has_text() {
        return Err(Error::BadStatic("channel doesnt have text"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
    }

    data.message_delete(channel_id, message_id).await?;
    data.media_link_delete_all(message_id.into_inner()).await?;

    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::MessageDelete {
                channel_id,
                message_id,
            },
        })
        .await?;
    }

    s.broadcast_channel(
        thread.id,
        auth.user.id,
        MessageSync::MessageDelete {
            channel_id,
            message_id,
        },
    )
    .await?;
    s.services().channels.invalidate(channel_id).await; // last version id, message count
    Ok(StatusCode::NO_CONTENT)
}

/// Message version list
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/message/{message_id}/version",
    params(
        PaginationQuery<MessageVerId>,
        ("channel_id", description = "Channel id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "success"),
    )
)]
async fn message_version_list(
    Path((channel_id, message_id)): Path<(ChannelId, MessageId)>,
    Query(q): Query<PaginationQuery<MessageVerId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let res = srv
        .messages
        .list_versions(channel_id, message_id, auth.user.id, q)
        .await?;
    Ok(Json(res))
}

/// Message version get
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/message/{message_id}/version/{version_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("message_id", description = "Message id"),
        ("version_id", description = "Version id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = Message, description = "success"),
    )
)]
async fn message_version_get(
    Path((channel_id, _message_id, version_id)): Path<(ChannelId, MessageId, MessageVerId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let message = srv
        .messages
        .get_version(channel_id, version_id, auth.user.id)
        .await?;
    Ok(Json(message))
}

/// Message version delete
///
/// Note that this endpoint allows deleting message versions, while message
/// moderate always requires the full permission
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/message/{message_id}/version/{version_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("message_id", description = "Message id"),
        ("version_id", description = "Version id")
    ),
    tags = [
        "message",
        "badge.perm-opt.MessageDelete",
        "badge.room-mfa-opt",
    ],
    responses(
        (status = NO_CONTENT, description = "delete message version success"),
    )
)]
async fn message_version_delete(
    Path((channel_id, message_id, version_id)): Path<(ChannelId, MessageId, MessageVerId)>,
    auth: Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    // FIXME: allow deleting your own messages without mfa
    if let Some(room_id) = thread.room_id {
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(Error::BadStatic("mfa required for this action"));
            }
        }
    }

    if !thread.ty.has_text() {
        return Err(Error::BadStatic("channel doesnt have text"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }

    let mut perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    if thread.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
    }
    perms.ensure(Permission::ViewChannel)?;

    let message = data
        .message_get(channel_id, message_id, auth.user.id)
        .await?;

    if message.latest_version.version_id == version_id {
        return Err(Error::BadStatic("cannot delete latest message version"));
    }

    let version = data
        .message_version_get(channel_id, version_id, auth.user.id)
        .await?;

    if !version.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete that message type"));
    }

    if message.author_id == auth.user.id {
        perms.add(Permission::MessageDelete);
    }
    perms.ensure(Permission::MessageDelete)?;

    data.message_version_delete(channel_id, version_id).await?;

    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::MessageVersionDelete {
                channel_id,
                message_id,
                version_id,
            },
        })
        .await?;
    }

    s.broadcast_channel(
        thread.id,
        auth.user.id,
        MessageSync::MessageVersionDelete {
            channel_id,
            message_id,
            version_id,
        },
    )
    .await?;

    // no need to invalidate channel cache as message count doesn't change
    Ok(StatusCode::NO_CONTENT)
}

/// Message moderate
///
/// Bulk remove, restore, or delete messages.
///
/// Deleting a message:
/// - Deleted messages remain visible to moderators (and sender, although there's no ui for this).
/// - Deleted messages cannot be restored by moderators (ask your local server admin if needed).
/// - Deleted messages are garbage collected after 7 days.
///
/// Removing a message:
/// - Removing a message hides it from all non-moderators and the sender.
/// - Removal is reversible via restoration, unlike deletion.
/// - Removed messages are never garbage collected.
/// - This is a "softer" form of deletion, intended for moderators you don't fully trust.
///
/// Permissions:
/// - `MessageDelete` allows deleting messages and viewing deleted messages.
/// - `MessageRemove` allows removing/restoring messages and viewing removed messages.
/// - Users can always delete (but not remove) their own messages.
#[utoipa::path(
    patch,
    path = "/channel/{channel_id}/message",
    params(("channel_id", description = "Channel id")),
    tags = [
        "message",
        "badge.perm-opt.MessageDelete",
        "badge.perm-opt.MessageRemove",
        "badge.room-mfa",
    ],
    responses((status = OK, description = "success")),
)]
async fn message_moderate(
    Path(channel_id): Path<ChannelId>,
    auth: Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<MessageModerate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;

    if json.delete.is_empty() && json.remove.is_empty() && json.restore.is_empty() {
        return Ok(StatusCode::OK);
    }

    let data = s.data();
    let srv = s.services();

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    // FIXME: allow deleting your own messages without mfa
    if let Some(room_id) = thread.room_id {
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(Error::BadStatic("mfa required for this action"));
            }
        }
    }

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !thread.ty.has_text() {
        return Err(Error::BadStatic("channel doesnt have text"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked && !perms.can_use_locked_threads() {
        return Err(Error::MissingPermissions);
    }

    if !json.delete.is_empty() {
        perms.ensure(Permission::MessageDelete)?;
        // TODO: fix n+1 query
        for id in &json.delete {
            let message = data.message_get(channel_id, *id, auth.user.id).await?;
            if !message.latest_version.message_type.is_deletable() {
                return Err(Error::BadStatic("cant delete one of the messages"));
            }
        }

        data.message_delete_bulk(channel_id, &json.delete).await?;
        for id in &json.delete {
            data.media_link_delete_all(id.into_inner()).await?;
        }

        if let Some(room_id) = thread.room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth.user.id,
                session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
                reason: reason.clone(),
                ty: AuditLogEntryType::MessageDeleteBulk {
                    channel_id,
                    message_ids: json.delete.clone(),
                },
            })
            .await?;
        }

        s.broadcast_channel(
            thread.id,
            auth.user.id,
            MessageSync::MessageDeleteBulk {
                channel_id,
                message_ids: json.delete.clone(),
            },
        )
        .await?;
    }

    if !json.remove.is_empty() {
        perms.ensure(Permission::MessageRemove)?;
        // TODO: fix n+1 query
        for id in &json.remove {
            let message = data.message_get(channel_id, *id, auth.user.id).await?;
            if !message.latest_version.message_type.is_deletable() {
                return Err(Error::BadStatic("cant remove one of the messages"));
            }
        }

        data.message_remove_bulk(channel_id, &json.remove).await?;

        if let Some(room_id) = thread.room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth.user.id,
                session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
                reason: reason.clone(),
                ty: AuditLogEntryType::MessageRemove {
                    channel_id,
                    message_ids: json.remove.clone(),
                },
            })
            .await?;
        }

        s.broadcast_channel(
            thread.id,
            auth.user.id,
            MessageSync::MessageRemove {
                channel_id,
                message_ids: json.remove.clone(),
            },
        )
        .await?;
    }

    if !json.restore.is_empty() {
        perms.ensure(Permission::MessageRemove)?;
        data.message_restore_bulk(channel_id, &json.restore).await?;

        if let Some(room_id) = thread.room_id {
            s.audit_log_append(AuditLogEntry {
                id: AuditLogEntryId::new(),
                room_id,
                user_id: auth.user.id,
                session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
                reason: reason.clone(),
                ty: AuditLogEntryType::MessageRestore {
                    channel_id,
                    message_ids: json.restore.clone(),
                },
            })
            .await?;
        }

        s.broadcast_channel(
            thread.id,
            auth.user.id,
            MessageSync::MessageRestore {
                channel_id,
                message_ids: json.restore.clone(),
            },
        )
        .await?;
    }

    srv.channels.invalidate(channel_id).await; // last version id, message count
    Ok(StatusCode::OK)
}

/// Message move (TODO)
///
/// Move messages from one thread to another. Requires `MessageMove` in both the
/// source and target thread.
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/move-messages",
    params(("channel_id", description = "Channel id")),
    tags = [
        "message",
        "badge.perm.MessageMove",
    ],
    responses((status = NO_CONTENT, description = "move success")),
)]
async fn message_move(
    Path(_channel_id): Path<ChannelId>,
    _auth: Auth,
    HeaderReason(_reason): HeaderReason,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<MessageMigrate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Message reply query
///
/// Get replies to a message
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/reply/{message_id}",
    params(
        RepliesQuery,
        ("channel_id", description = "Channel id"),
        ("message_id", description = "Message id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List thread messages success"),
    ),
)]
async fn message_reply_query(
    Path((channel_id, message_id)): Path<(ChannelId, MessageId)>,
    Query(q): Query<RepliesQuery>,
    Query(pagination): Query<PaginationQuery<MessageId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    q.validate()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let res = srv
        .messages
        .list_replies(channel_id, Some(message_id), auth.user.id, q, pagination)
        .await?;
    Ok(Json(res))
}

/// Message reply roots
///
/// Get messages that don't reply to any other messages
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/reply",
    params(
        RepliesQuery,
        ("channel_id", description = "Channel id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List thread messages success"),
    ),
)]
async fn message_reply_roots(
    Path((channel_id,)): Path<(ChannelId,)>,
    Query(q): Query<RepliesQuery>,
    Query(pagination): Query<PaginationQuery<MessageId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    q.validate()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let res = srv
        .messages
        .list_replies(channel_id, None, auth.user.id, q, pagination)
        .await?;
    Ok(Json(res))
}

/// Pin create
///
/// - Newly pinned messages are pinned to the top (position 0).
/// - There can be a maximum of 1024 pinned messages.
#[utoipa::path(
    put,
    path = "/channel/{channel_id}/pin/{message_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("message_id", description = "Message id")
    ),
    tags = [
        "message",
        "badge.perm.MessagePin",
    ],
    responses(
        (status = OK, description = "success"),
    ),
)]
async fn message_pin_create(
    Path((channel_id, message_id)): Path<(ChannelId, MessageId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = thread.room_id {
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(Error::BadStatic("mfa required for this action"));
            }
        }
    }

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::MessagePin)?;

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !thread.ty.has_text() {
        return Err(Error::BadStatic("channel doesnt have text"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }

    data.message_pin_create(channel_id, message_id).await?;

    let message = data
        .message_get(channel_id, message_id, auth.user.id)
        .await?;

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::MessageUpdate { message },
    )
    .await?;

    let notice_message_id = data
        .message_create(DbMessageCreate {
            channel_id,
            attachment_ids: vec![],
            author_id: auth.user.id,
            embeds: vec![],
            message_type: MessageType::MessagePinned(MessagePin {
                pinned_message_id: message_id,
            }),
            edited_at: None,
            created_at: None,
            mentions: Default::default(),
        })
        .await?;
    let mut notice_message = data
        .message_get(channel_id, notice_message_id, auth.user.id)
        .await?;

    let user_id = auth.user.id;
    let tm = data.thread_member_get(channel_id, user_id).await;
    if tm.is_err() || tm.is_ok_and(|tm| tm.membership == ThreadMembership::Leave) {
        data.thread_member_put(channel_id, user_id, ThreadMemberPut::default())
            .await?;
        let thread_member = data.thread_member_get(channel_id, user_id).await?;
        let msg = MessageSync::ThreadMemberUpsert {
            member: thread_member,
        };
        s.broadcast_channel(channel_id, user_id, msg).await?;
    }

    s.presign_message(&mut notice_message).await?;
    srv.channels.invalidate(channel_id).await; // message count
    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::MessageCreate {
            message: notice_message,
        },
    )
    .await?;

    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason: reason.clone(),
            ty: AuditLogEntryType::MessagePin {
                channel_id,
                message_id,
            },
        })
        .await?;
    }

    Ok(StatusCode::OK)
}

/// Pin delete
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/pin/{message_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("message_id", description = "Message id")
    ),
    tags = [
        "message",
        "badge.perm.MessagePin",
    ],
    responses(
        (status = OK, description = "success"),
    ),
)]
async fn message_pin_delete(
    Path((channel_id, message_id)): Path<(ChannelId, MessageId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if let Some(room_id) = thread.room_id {
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(Error::BadStatic("mfa required for this action"));
            }
        }
    }

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::MessagePin)?;

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !thread.ty.has_text() {
        return Err(Error::BadStatic("channel doesnt have text"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }

    s.data().message_pin_delete(channel_id, message_id).await?;

    let message = s
        .data()
        .message_get(channel_id, message_id, auth.user.id)
        .await?;

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::MessageUpdate { message },
    )
    .await?;

    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason: reason.clone(),
            ty: AuditLogEntryType::MessageUnpin {
                channel_id,
                message_id,
            },
        })
        .await?;
    }

    Ok(StatusCode::OK)
}

/// Pin reorder
#[utoipa::path(
    patch,
    path = "/channel/{channel_id}/pin",
    params(
        ("channel_id", description = "Channel id"),
    ),
    tags = [
        "message",
        "badge.perm.MessagePin",
        "badge.room-mfa",
    ],
    responses(
        (status = OK, description = "Reorder pinned messages success"),
    ),
)]
async fn message_pin_reorder(
    Path((channel_id,)): Path<(ChannelId,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<PinsReorder>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;
    let srv = s.services();
    let data = s.data();

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if let Some(room_id) = thread.room_id {
        let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
        if room.security.require_mfa {
            let user = srv.users.get(auth.user.id, None).await?;
            let totp = data.auth_totp_get(user.id).await?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(Error::BadStatic("mfa required for this action"));
            }
        }
    }

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::MessagePin)?;

    let thread = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !thread.ty.has_text() {
        return Err(Error::BadStatic("channel doesnt have text"));
    }
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }

    s.data()
        .message_pin_reorder(channel_id, json.clone())
        .await?;

    // broadcast update for all affected messages
    for item in json.messages {
        let message = s
            .data()
            .message_get(channel_id, item.id, auth.user.id)
            .await?;
        s.broadcast_channel(
            channel_id,
            auth.user.id,
            MessageSync::MessageUpdate { message },
        )
        .await?;
    }

    if let Some(room_id) = thread.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
            reason: reason.clone(),
            ty: AuditLogEntryType::MessagePinReorder { channel_id },
        })
        .await?;
    }

    Ok(StatusCode::OK)
}

/// Pin list
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/pin",
    params(
        PaginationQuery<MessageId>,
        ("channel_id", description = "Channel id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List pinned messages success"),
    ),
)]
async fn message_pin_list(
    Path(channel_id): Path<ChannelId>,
    Query(q): Query<PaginationQuery<MessageId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    let res = srv.messages.list_pins(channel_id, auth.user.id, q).await?;
    Ok(Json(res))
}

/// Message list deleted
///
/// Paginate deleted messages in a thread
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/message/deleted",
    params(PaginationQuery<MessageId>, ("channel_id", description = "Channel id")),
    tags = ["message", "badge.perm-opt.MessageDelete"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "success"),
    )
)]
async fn message_list_deleted(
    Path((channel_id,)): Path<(ChannelId,)>,
    Query(q): Query<PaginationQuery<MessageId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::MessageDelete)?;
    let res = srv
        .messages
        .list_deleted(channel_id, auth.user.id, q)
        .await?;
    Ok(Json(res))
}

/// Message list removed
///
/// Paginate removed messages in a thread
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/message/removed",
    params(PaginationQuery<MessageId>, ("channel_id", description = "Channel id")),
    tags = ["message", "badge.perm-opt.MessageRemove"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "success"),
    )
)]
async fn message_list_removed(
    Path((channel_id,)): Path<(ChannelId,)>,
    Query(q): Query<PaginationQuery<MessageId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::MessageRemove)?;
    let res = srv
        .messages
        .list_removed(channel_id, auth.user.id, q)
        .await?;
    Ok(Json(res))
}

/// Message list atom/rss (TODO)
///
/// Get an atom or rss feed of messages for this channel
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/message.atom",
    params(
        ("channel_id", description = "Channel id"),
        PaginationQuery<ChannelId>
    ),
    tags = ["message"],
)]
pub async fn message_list_atom(
    Path(_channel_id): Path<ChannelId>,
    Query(_pagination): Query<PaginationQuery<ChannelId>>,
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Nudge (TODO)
///
/// Nudge a user. Can only be used in dms or gdms. Can only be called once every 5 minutes per user.
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/nudge",
    params(("channel_id", description = "Channel id")),
    tags = ["message"],
)]
pub async fn message_nudge(
    Path(_channel_id): Path<ChannelId>,
    _auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(message_create))
        .routes(routes!(message_get))
        .routes(routes!(message_list))
        .routes(routes!(message_list_deleted))
        .routes(routes!(message_list_removed))
        .routes(routes!(message_list_atom))
        .routes(routes!(message_context))
        .routes(routes!(message_edit))
        .routes(routes!(message_delete))
        .routes(routes!(message_version_list))
        .routes(routes!(message_version_get))
        .routes(routes!(message_version_delete))
        .routes(routes!(message_reply_query))
        .routes(routes!(message_reply_roots))
        .routes(routes!(message_moderate))
        .routes(routes!(message_move))
        .routes(routes!(message_pin_create))
        .routes(routes!(message_pin_delete))
        .routes(routes!(message_pin_reorder))
        .routes(routes!(message_pin_list))
        .routes(routes!(message_nudge))
}
