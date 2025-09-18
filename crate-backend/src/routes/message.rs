use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{AuditLogEntry, AuditLogEntryId, AuditLogEntryType, PaginationDirection};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Error,
    types::{
        Message, MessageCreate, MessageId, MessagePatch, MessageSync, MessageVerId,
        PaginationQuery, PaginationResponse, Permission, ThreadId,
    },
    ServerState,
};

use super::util::{Auth, HeaderIdempotencyKey, HeaderReason};
use crate::error::Result;

/// Message create
///
/// Send a message to a thread
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/message",
    params(("thread_id", description = "Thread id")),
    tags = ["message"],
    responses(
        (status = CREATED, body = Message, description = "Create message success"),
    )
)]
async fn message_create(
    Path((thread_id,)): Path<(ThreadId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    HeaderIdempotencyKey(nonce): HeaderIdempotencyKey,
    Json(json): Json<MessageCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();

    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
        perms.ensure(Permission::ThreadLock)?;
    }

    let message = srv
        .messages
        .create(thread_id, auth_user.id, reason, nonce, json)
        .await?;
    Ok((StatusCode::CREATED, Json(message)))
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema, IntoParams)]
struct ContextQuery {
    to_start: Option<MessageId>,
    to_end: Option<MessageId>,
    limit: Option<u16>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
struct ContextResponse {
    items: Vec<Message>,
    total: u64,
    has_after: bool,
    has_before: bool,
}

/// Message get context
///
/// More efficient than calling List messages twice
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/context/{message_id}",
    params(
        ContextQuery,
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = ContextResponse, description = "List thread messages success"),
    )
)]
async fn message_context(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Query(q): Query<ContextQuery>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let limit = q.limit.unwrap_or(10);
    if limit > 1024 {
        return Err(Error::BadStatic("limit too big"));
    }
    let before_q = PaginationQuery {
        from: Some(message_id),
        to: q.to_start,
        dir: Some(PaginationDirection::B),
        limit: Some(limit),
    };
    let before = data.message_list(thread_id, auth_user.id, before_q).await?;
    let after_q = PaginationQuery {
        from: Some(message_id),
        to: q.to_end,
        dir: Some(PaginationDirection::F),
        limit: Some(limit),
    };
    let after = data.message_list(thread_id, auth_user.id, after_q).await?;
    let message = data
        .message_get(thread_id, message_id, auth_user.id)
        .await?;
    let mut res = ContextResponse {
        items: before
            .items
            .into_iter()
            .chain([message])
            .chain(after.items.into_iter())
            .collect(),
        total: after.total,
        has_after: after.has_more,
        has_before: before.has_more,
    };
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Messages list
///
/// Paginate messages in a thread
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message",
    params(PaginationQuery<MessageId>, ("thread_id", description = "Thread id")),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List thread messages success"),
    )
)]
async fn message_list(
    Path((thread_id,)): Path<(ThreadId,)>,
    Query(q): Query<PaginationQuery<MessageId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut res = data.message_list(thread_id, auth_user.id, q).await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Message get
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, body = Message, description = "List thread messages success"),
    )
)]
async fn message_get(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut message = data
        .message_get(thread_id, message_id, auth_user.id)
        .await?;
    s.presign_message(&mut message).await?;
    Ok(Json(message))
}

/// Message edit
#[utoipa::path(
    patch,
    path = "/thread/{thread_id}/message/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, body = Message, description = "edit message success"),
        (status = NOT_MODIFIED, description = "no change"),
    )
)]
async fn message_edit(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<MessagePatch>,
) -> Result<(StatusCode, Json<Message>)> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
        perms.ensure(Permission::ThreadLock)?;
    }

    let (status, message) = srv
        .messages
        .edit(thread_id, message_id, auth_user.id, reason, json)
        .await?;
    Ok((status, Json(message)))
}

/// Message delete (TEMP?)
///
/// Note that this endpoint allows deleting your own messages, while message
/// moderate always requires the full permission
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/message/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = NO_CONTENT, description = "delete message success"),
    )
)]
async fn message_delete(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<StatusCode> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let mut perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    let message = data
        .message_get(thread_id, message_id, auth_user.id)
        .await?;
    if !message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete that message"));
    }
    if message.author_id == auth_user.id {
        perms.add(Permission::MessageEdit);
    }
    perms.ensure(Permission::MessageDelete)?;
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

    data.message_delete(thread_id, message_id).await?;
    data.media_link_delete_all(message_id.into_inner()).await?;

    if let Some(room_id) = thread.room_id {
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::MessageDelete {
                thread_id,
                message_id,
            },
        })
        .await?;
    }

    s.broadcast_thread(
        thread.id,
        auth_user.id,
        MessageSync::MessageDelete {
            room_id: thread.room_id,
            thread_id,
            message_id,
        },
    )
    .await?;
    s.services().threads.invalidate(thread_id).await; // last version id, message count
    Ok(StatusCode::NO_CONTENT)
}

/// Message version list
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}/version",
    params(
        PaginationQuery<MessageVerId>,
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "success"),
    )
)]
async fn message_version_list(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Query(q): Query<PaginationQuery<MessageVerId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<PaginationResponse<Message>>> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut res = data
        .message_version_list(thread_id, message_id, auth_user.id, q)
        .await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Message version get
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}/version/{version_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("version_id", description = "Version id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = Message, description = "success"),
    )
)]
async fn message_version_get(
    Path((thread_id, _message_id, version_id)): Path<(ThreadId, MessageId, MessageVerId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<Message>> {
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut message = data
        .message_version_get(thread_id, version_id, auth_user.id)
        .await?;
    s.presign_message(&mut message).await?;
    Ok(Json(message))
}

/// Message version delete
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/message/{message_id}/version/{version_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("version_id", description = "Version id"),
    ),
    tags = ["message"],
    responses(
        (status = NO_CONTENT, description = "delete message success"),
    )
)]
async fn message_version_delete(
    Path((thread_id, _message_id, version_id)): Path<(ThreadId, MessageId, MessageVerId)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<Json<()>> {
    auth_user.ensure_unsuspended()?;
    let data = s.data();
    let srv = s.services();
    let mut perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    let message = data
        .message_version_get(thread_id, version_id, auth_user.id)
        .await?;
    if !message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete this message type"));
    }
    if message.author_id == auth_user.id {
        perms.add(Permission::MessageDelete);
    }
    perms.ensure(Permission::MessageDelete)?;

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

    data.message_version_delete(thread_id, version_id).await?;

    let thread = s
        .services()
        .threads
        .get(thread_id, Some(auth_user.id))
        .await?;
    if let Some(room_id) = thread.room_id {
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::MessageVersionDelete {
                thread_id,
                message_id: message.id,
                version_id,
            },
        })
        .await?;
    }

    s.services().threads.invalidate(thread_id).await; // last version id, message count
    Ok(Json(()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Validate)]
struct MessageBulkDelete {
    /// which messages to delete
    #[serde(default)]
    #[validate(length(min = 1, max = 128))]
    message_ids: Vec<MessageId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Validate)]
struct MessageMigrate {
    /// which messages to move
    #[serde(default)]
    #[validate(length(min = 1, max = 128))]
    message_ids: Vec<MessageId>,

    /// must be in same room (for now...)
    target_id: ThreadId,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
struct MessageModerate {
    /// which messages to delete
    #[serde(default)]
    #[validate(length(min = 1, max = 128))]
    delete: Vec<MessageId>,

    /// which messages to remove
    #[serde(default)]
    #[validate(length(min = 1, max = 128))]
    remove: Vec<MessageId>,

    /// which messages to restore
    #[serde(default)]
    #[validate(length(min = 1, max = 128))]
    restore: Vec<MessageId>,
}

/// Message moderate (WIP)
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
/// - Removal is reversable via restoration, unlike deletion.
/// - Removed messages are never garbage collected.
/// - There is (will be) an endpoint for deleting all removed messages.
/// - This is a "softer" form of deletion, intended for moderators you don't fully trust.
///
/// Permissions:
/// - `MessageDelete` allows deleting messages and viewing deleted messages.
/// - `MessageRemove` allows removing/restoring messages and viewing removed messages.
/// - `Omnescience` allows viewing deleted and removed messages.
/// - Users always have `MessageDelete` for their own messages.
#[utoipa::path(
    patch,
    path = "/thread/{thread_id}/message",
    params(("thread_id", description = "Thread id")),
    tags = ["message"],
    responses((status = OK, description = "success")),
)]
async fn message_moderate(
    Path(thread_id): Path<ThreadId>,
    Auth(auth_user): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<MessageModerate>,
) -> Result<StatusCode> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;

    if !json.remove.is_empty() || !json.restore.is_empty() {
        return Err(Error::BadStatic(
            "remove and restore are not implemented yet",
        ));
    }

    if json.delete.is_empty() {
        // nothing to do
        return Ok(StatusCode::OK);
    }

    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user.id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::MessageDelete)?;

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

    // TODO: fix n+1 query
    for id in &json.delete {
        let message = data.message_get(thread_id, *id, auth_user.id).await?;
        if !message.message_type.is_deletable() {
            return Err(Error::BadStatic("cant delete one of the messages"));
        }
    }

    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    data.message_delete_bulk(thread_id, &json.delete).await?;
    for id in &json.delete {
        data.media_link_delete_all(id.into_inner()).await?;
    }

    if let Some(room_id) = thread.room_id {
        data.audit_logs_room_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth_user.id,
            session_id: None,
            reason: reason.clone(),
            ty: AuditLogEntryType::MessageDeleteBulk {
                thread_id,
                message_ids: json.delete.clone(),
            },
        })
        .await?;
    }

    s.broadcast_thread(
        thread.id,
        auth_user.id,
        MessageSync::MessageDeleteBulk {
            thread_id,
            message_ids: json.delete,
        },
    )
    .await?;
    srv.threads.invalidate(thread_id).await; // last version id, message count
    Ok(StatusCode::OK)
}

/// Message move (TODO)
///
/// Move messages from one thread to another. Requires `MessageMove` in both the
/// source and target thread.
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/migrate",
    params(("thread_id", description = "Thread id")),
    tags = ["message"],
    responses((status = NO_CONTENT, description = "move success")),
)]
async fn message_migrate(
    Path(_thread_id): Path<ThreadId>,
    Auth(_user_id): Auth,
    HeaderReason(_reason): HeaderReason,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<MessageMigrate>,
) -> Result<()> {
    Err(Error::Unimplemented)
}

// TODO: move these structs to common
#[derive(Debug, Default, Serialize, Deserialize, ToSchema, IntoParams, Validate)]
struct RepliesQuery {
    #[serde(flatten)]
    q: PaginationQuery<MessageId>,

    /// how deeply to fetch replies
    #[serde(default = "fn_one")]
    #[validate(range(min = 1, max = 8))]
    depth: u16,

    /// how many replies to fetch per branch
    breadth: Option<u16>,
}

/// always returns one
fn fn_one() -> u16 {
    1
}

/// Message replies
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/reply/{message_id}",
    params(
        RepliesQuery,
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List thread messages success"),
    ),
)]
async fn message_replies(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Query(q): Query<RepliesQuery>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    q.validate()?;
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut res = data
        .message_replies(
            thread_id,
            Some(message_id),
            auth_user.id,
            q.depth,
            q.breadth,
            q.q,
        )
        .await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Message roots
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/reply",
    params(
        RepliesQuery,
        ("thread_id", description = "Thread id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "List thread messages success"),
    ),
)]
async fn message_roots(
    Path((thread_id,)): Path<(ThreadId,)>,
    Query(q): Query<RepliesQuery>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    q.validate()?;
    let data = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, thread_id)
        .await?;
    perms.ensure_view()?;
    let mut res = data
        .message_replies(thread_id, None, auth_user.id, q.depth, q.breadth, q.q)
        .await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(message_create))
        .routes(routes!(message_get))
        .routes(routes!(message_list))
        .routes(routes!(message_context))
        .routes(routes!(message_edit))
        .routes(routes!(message_delete))
        .routes(routes!(message_version_list))
        .routes(routes!(message_version_get))
        .routes(routes!(message_version_delete))
        .routes(routes!(message_replies))
        .routes(routes!(message_roots))
        .routes(routes!(message_moderate))
        .routes(routes!(message_migrate))
}
