use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::PaginationDirection;
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

/// Create a message
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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    HeaderIdempotencyKey(nonce): HeaderIdempotencyKey,
    Json(json): Json<MessageCreate>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let message = srv
        .messages
        .create(thread_id, user_id, reason, nonce, json)
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

/// Get context for message
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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let limit = q.limit.unwrap_or(10);
    if limit > 100 {
        return Err(Error::BadStatic("limit too big"));
    }
    let before_q = PaginationQuery {
        from: Some(message_id),
        to: q.to_start,
        dir: Some(PaginationDirection::B),
        limit: Some(limit),
    };
    let before = data.message_list(thread_id, user_id, before_q).await?;
    let after_q = PaginationQuery {
        from: Some(message_id),
        to: q.to_end,
        dir: Some(PaginationDirection::F),
        limit: Some(limit),
    };
    let after = data.message_list(thread_id, user_id, after_q).await?;
    let message = data.message_get(thread_id, message_id, user_id).await?;
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

/// List messages in a thread
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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut res = data.message_list(thread_id, user_id, q).await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Get a message
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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut message = data.message_get(thread_id, message_id, user_id).await?;
    s.presign_message(&mut message).await?;
    Ok(Json(message))
}

/// edit a message
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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<MessagePatch>,
) -> Result<(StatusCode, Json<Message>)> {
    let (status, message) = s
        .services()
        .messages
        .edit(thread_id, message_id, user_id, reason, json)
        .await?;
    Ok((status, Json(message)))
}

/// Delete message
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
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<StatusCode> {
    let data = s.data();
    let mut perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let message = data.message_get(thread_id, message_id, user_id).await?;
    if !message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete that message"));
    }
    if message.author_id == user_id {
        perms.add(Permission::MessageEdit);
    }
    perms.ensure(Permission::MessageDelete)?;
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    data.message_delete(thread_id, message_id).await?;
    data.media_link_delete_all(message_id.into_inner()).await?;
    s.broadcast_thread(
        thread.id,
        user_id,
        reason,
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

/// List message versions
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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<PaginationResponse<Message>>> {
    let data = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut res = data
        .message_version_list(thread_id, message_id, user_id, q)
        .await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Get message version
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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<Message>> {
    let data = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut message = data
        .message_version_get(thread_id, version_id, user_id)
        .await?;
    s.presign_message(&mut message).await?;
    Ok(Json(message))
}

/// Delete message version
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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    let data = s.data();
    let mut perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let message = data
        .message_version_get(thread_id, version_id, user_id)
        .await?;
    if !message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete this message type"));
    }
    if message.author_id == user_id {
        perms.add(Permission::MessageDelete);
    }
    perms.ensure(Permission::MessageDelete)?;
    data.message_version_delete(thread_id, version_id).await?;
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

/// Message delete bulk
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/messages/delete",
    params(("thread_id", description = "Thread id")),
    tags = ["message"],
    responses((status = NO_CONTENT, description = "bulk delete success")),
)]
#[deprecated = "use message_moderate"]
async fn message_delete_bulk(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<MessageBulkDelete>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let data = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    for id in &json.message_ids {
        let message = data.message_get(thread_id, *id, user_id).await?;
        if !message.message_type.is_deletable() {
            return Err(Error::BadStatic("cant delete that message"));
        }
        perms.ensure(Permission::MessageDelete)?;
    }
    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    data.message_delete_bulk(thread_id, &json.message_ids)
        .await?;
    for id in &json.message_ids {
        data.media_link_delete_all(id.into_inner()).await?;
    }
    s.broadcast_thread(
        thread.id,
        user_id,
        reason,
        MessageSync::MessageDeleteBulk {
            thread_id,
            message_ids: json.message_ids,
        },
    )
    .await?;
    s.services().threads.invalidate(thread_id).await; // last version id, message count
    Ok(StatusCode::NO_CONTENT)
}

/// Message moderate (WIP)
#[utoipa::path(
    patch,
    path = "/thread/{thread_id}/message",
    params(("thread_id", description = "Thread id")),
    tags = ["message"],
    responses((status = OK, description = "success")),
)]
async fn message_moderate(
    Path(thread_id): Path<ThreadId>,
    Auth(user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<MessageModerate>,
) -> Result<StatusCode> {
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
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::MessageDelete)?;

    for id in &json.delete {
        let message = data.message_get(thread_id, *id, user_id).await?;
        if !message.message_type.is_deletable() {
            return Err(Error::BadStatic("cant delete one of the messages"));
        }
    }

    let thread = s.services().threads.get(thread_id, Some(user_id)).await?;
    data.message_delete_bulk(thread_id, &json.delete).await?;
    for id in &json.delete {
        data.media_link_delete_all(id.into_inner()).await?;
    }
    s.broadcast_thread(
        thread.id,
        user_id,
        reason,
        MessageSync::MessageDeleteBulk {
            thread_id,
            message_ids: json.delete,
        },
    )
    .await?;
    s.services().threads.invalidate(thread_id).await; // last version id, message count
    Ok(StatusCode::OK)
}

/// Message move (TODO)
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
    todo!()
}

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
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    q.validate()?;
    let data = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut res = data
        .message_replies(thread_id, message_id, user_id, q.depth, q.breadth, q.q)
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
        .routes(routes!(message_delete_bulk))
        .routes(routes!(message_replies))
        .routes(routes!(message_moderate))
        .routes(routes!(message_migrate))
}
