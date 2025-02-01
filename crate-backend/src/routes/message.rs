use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use types::{util::Diff, PaginationDirection};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    error::Error,
    types::{
        MediaLinkType, Message, MessageCreate, MessageCreateRequest, MessageId, MessagePatch,
        MessageSync, MessageType, MessageVerId, PaginationQuery, PaginationResponse, Permission,
        ThreadId,
    },
    ServerState,
};

use super::util::Auth;
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
    Json(json): Json<MessageCreateRequest>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::MessageCreate)?;
    if !json.attachments.is_empty() {
        perms.ensure(Permission::MessageFilesEmbeds)?;
    }
    // TODO: everyone can set override_name, but it's meant to be temporary so its probably fine
    if json.content.as_ref().is_none_or(|s| s.is_empty()) && json.attachments.is_empty() {
        return Err(Error::BadStatic(
            "at least one of content, attachments, or embeds must be defined",
        ));
    }
    let attachment_ids: Vec<_> = json.attachments.into_iter().map(|r| r.id).collect();
    for id in &attachment_ids {
        data.media_select(*id).await?;
        let existing = data.media_link_select(*id).await?;
        if !existing.is_empty() {
            return Err(Error::BadStatic("cant reuse media"));
        }
    }
    let message_id = data
        .message_create(MessageCreate {
            thread_id,
            content: json.content,
            attachment_ids: attachment_ids.clone(),
            author_id: user_id,
            message_type: MessageType::Default,
            metadata: json.metadata,
            reply_id: json.reply_id,
            override_name: json.override_name,
        })
        .await?;
    let message_uuid = message_id.into_inner();
    for id in &attachment_ids {
        data.media_link_insert(*id, message_uuid, MediaLinkType::Message)
            .await?;
        data.media_link_insert(*id, message_uuid, MediaLinkType::MessageVersion)
            .await?;
    }
    let mut message = data.message_get(thread_id, message_id).await?;
    for media in &mut message.attachments {
        media.url = s.presign(&media.url).await?;
    }
    message.nonce = json.nonce;
    let msg = MessageSync::UpsertMessage {
        message: message.clone(),
    };
    s.broadcast(msg)?;
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

/// Get context for messages in a thread
///
/// More efficient than calling List Message twice
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
        (status = OK, body = PaginationResponse<Message>, description = "List thread messages success"),
    )
)]
async fn message_context(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Query(q): Query<ContextQuery>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = data.permission_thread_get(user_id, thread_id).await?;
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
    let before = data.message_list(thread_id, before_q).await?;
    let after_q = PaginationQuery {
        from: Some(message_id),
        to: q.to_end,
        dir: Some(PaginationDirection::F),
        limit: Some(limit),
    };
    let after = data.message_list(thread_id, after_q).await?;
    let message = data.message_get(thread_id, message_id).await?;
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
        for media in &mut message.attachments {
            media.url = s.presign(&media.url).await?;
        }
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
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut res = data.message_list(thread_id, q).await?;
    for message in &mut res.items {
        for media in &mut message.attachments {
            media.url = s.presign(&media.url).await?;
        }
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
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut message = data.message_get(thread_id, message_id).await?;
    for media in &mut message.attachments {
        media.url = s.presign(&media.url).await?;
    }
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
    Json(patch): Json<MessagePatch>,
) -> Result<(StatusCode, Json<Message>)> {
    let data = s.data();
    let mut perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let message = data.message_get(thread_id, message_id).await?;
    if !message.message_type.is_editable() {
        return Err(Error::BadStatic("cant edit that message"));
    }
    if message.author.id == user_id {
        perms.add(Permission::MessageEdit);
    }
    perms.ensure(Permission::MessageEdit)?;
    if patch.content.is_none() && patch.attachments.as_ref().is_some_and(|a| a.is_empty()) {
        return Err(Error::BadStatic(
            "at least one of content, attachments, or embeds must be defined",
        ));
    }
    if !patch.attachments.as_ref().is_some_and(|a| !a.is_empty()) {
        perms.ensure(Permission::MessageFilesEmbeds)?;
    }
    if !patch.changes(&message) {
        return Ok((StatusCode::NOT_MODIFIED, Json(message)));
    }
    let attachment_ids: Vec<_> = patch
        .attachments
        .map(|ats| ats.into_iter().map(|r| r.id).collect())
        .unwrap_or_else(|| {
            message
                .attachments
                .into_iter()
                .map(|media| media.id)
                .collect()
        });
    for id in &attachment_ids {
        data.media_select(*id).await?;
        let existing = data.media_link_select(*id).await?;
        let has_link = existing.iter().any(|i| {
            i.link_type == MediaLinkType::Message && i.target_id == message_id.into_inner()
        });
        if !has_link {
            return Err(Error::BadStatic("cant reuse media"));
        }
    }
    let version_id = data
        .message_update(
            thread_id,
            message_id,
            MessageCreate {
                thread_id,
                content: patch.content.unwrap_or(message.content),
                attachment_ids: attachment_ids.clone(),
                author_id: user_id,
                message_type: MessageType::Default,
                metadata: patch.metadata.unwrap_or(message.metadata),
                reply_id: patch.reply_id.unwrap_or(message.reply_id),
                override_name: patch.override_name.unwrap_or(message.override_name),
            },
        )
        .await?;
    let version_uuid = version_id.into_inner();
    for id in &attachment_ids {
        data.media_link_insert(*id, version_uuid, MediaLinkType::MessageVersion)
            .await?;
    }
    let mut message = data
        .message_version_get(thread_id, message_id, version_id)
        .await?;
    for media in &mut message.attachments {
        media.url = s.presign(&media.url).await?;
    }
    s.broadcast(MessageSync::UpsertMessage {
        message: message.clone(),
    })?;
    Ok((StatusCode::CREATED, Json(message)))
}

/// delete a message
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
    State(s): State<Arc<ServerState>>,
) -> Result<StatusCode> {
    let data = s.data();
    let mut perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let message = data.message_get(thread_id, message_id).await?;
    if !message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete that message"));
    }
    if message.author.id == user_id {
        perms.add(Permission::MessageEdit);
    }
    perms.ensure(Permission::MessageDelete)?;
    let thread = data.thread_get(thread_id, None).await?;
    data.message_delete(thread_id, message_id).await?;
    data.media_link_delete_all(message_id.into_inner()).await?;
    s.broadcast(MessageSync::DeleteMessage {
        room_id: thread.room_id,
        thread_id,
        message_id,
    })?;
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
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut res = data.message_version_list(thread_id, message_id, q).await?;
    for message in &mut res.items {
        for media in &mut message.attachments {
            media.url = s.presign(&media.url).await?;
        }
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
    Path((thread_id, message_id, version_id)): Path<(ThreadId, MessageId, MessageVerId)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<Message>> {
    let data = s.data();
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut message = data
        .message_version_get(thread_id, message_id, version_id)
        .await?;
    for media in &mut message.attachments {
        media.url = s.presign(&media.url).await?;
    }
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
    Path((thread_id, message_id, version_id)): Path<(ThreadId, MessageId, MessageVerId)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    let data = s.data();
    let mut perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let message = data
        .message_version_get(thread_id, message_id, version_id)
        .await?;
    if !message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete this message type"));
    }
    if message.author.id == user_id {
        perms.add(Permission::MessageDelete);
    }
    perms.ensure(Permission::MessageDelete)?;
    data.message_version_delete(thread_id, message_id, version_id)
        .await?;
    Ok(Json(()))
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
}
