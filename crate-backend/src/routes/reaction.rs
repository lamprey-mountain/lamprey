use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::reaction::{ReactionKey, ReactionListItem};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageId, MessageSync, PaginationQuery,
    PaginationResponse, Permission, ThreadId, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::{Auth, HeaderReason};
use crate::error::Result;
use crate::{Error, ServerState};

/// Reaction add
///
/// Add a reaction to a message.
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/message/{message_id}/reaction/{key}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("key", description = "Reaction key"),
    ),
    tags = ["reaction"],
    responses(
        (status = CREATED, description = "new reaction created"),
        (status = OK, description = "already exists"),
    )
)]
async fn reaction_add(
    Path((thread_id, message_id, key)): Path<(ThreadId, MessageId, ReactionKey)>,
    Auth(auth_user): Auth,
    HeaderReason(_reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ReactionAdd)?;
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }
    data.reaction_put(auth_user.id, thread_id, message_id, key.clone())
        .await?;
    s.broadcast_thread(
        thread_id,
        auth_user.id,
        MessageSync::ReactionCreate {
            thread_id,
            user_id: auth_user.id,
            message_id,
            key,
        },
    )
    .await?;
    Ok(StatusCode::OK)
}

/// Reaction remove
///
/// Remove a reaction from a message.
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/message/{message_id}/reaction/{key}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("key", description = "Reaction key"),
    ),
    tags = ["reaction"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn reaction_remove(
    Path((thread_id, message_id, key)): Path<(ThreadId, MessageId, ReactionKey)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ReactionAdd)?;
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }
    data.reaction_delete(auth_user.id, thread_id, message_id, key.clone())
        .await?;
    s.broadcast_thread(
        thread_id,
        auth_user.id,
        MessageSync::ReactionDelete {
            thread_id,
            user_id: auth_user.id,
            message_id,
            key,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Reaction purge
///
/// Remove all reactions from a message.
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/message/{message_id}/reaction",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
    ),
    tags = ["reaction"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn reaction_purge(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ReactionClear)?;
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }
    data.reaction_purge(thread_id, message_id).await?;

    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if let Some(room_id) = thread.room_id {
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::ReactionPurge {
                thread_id,
                message_id,
            },
        })
        .await?;
    }

    s.broadcast_thread(
        thread_id,
        auth_user.id,
        MessageSync::ReactionPurge {
            thread_id,
            message_id,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Reaction list
///
/// List message reactions for a specific emoji.
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}/reaction/{key}",
    params(
        PaginationQuery<UserId>,
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("key", description = "Reaction key"),
    ),
    tags = ["reaction"],
    responses(
        (status = OK, body = PaginationResponse<ReactionListItem>, description = "success"),
    )
)]
async fn reaction_list(
    Path((thread_id, message_id, key)): Path<(ThreadId, MessageId, ReactionKey)>,
    Auth(auth_user): Auth,
    Query(q): Query<PaginationQuery<UserId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    let list = data.reaction_list(thread_id, message_id, key, q).await?;
    Ok(Json(list))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(reaction_add))
        .routes(routes!(reaction_remove))
        .routes(routes!(reaction_purge))
        .routes(routes!(reaction_list))
}
