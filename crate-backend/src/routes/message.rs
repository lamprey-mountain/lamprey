use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use linkify::LinkFinder;
use serde::{Deserialize, Serialize};
use types::{util::Diff, MessageDefaultMarkdown, PaginationDirection, ThreadMembership};
use url::Url;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Error,
    types::{
        DbMessageCreate, MediaLinkType, Message, MessageCreate, MessageId, MessagePatch,
        MessageSync, MessageType, MessageVerId, PaginationQuery, PaginationResponse, Permission,
        ThreadId,
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
    json.validate()?;
    let data = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::MessageCreate)?;
    if !json.attachments.is_empty() {
        perms.ensure(Permission::MessageAttachments)?;
    }
    // if !json.embeds.is_empty() {
    //     perms.ensure(Permission::MessageEmbeds)?;
    // }
    // TODO: everyone can set override_name, but it's meant to be temporary so its probably fine
    // TODO: move this to validation
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
    let body = if json.use_new_text_formatting {
        return Err(Error::Unimplemented);
    } else {
        MessageDefaultMarkdown {
            content: json.content,
            attachments: vec![],
            embeds: vec![],
            metadata: json.metadata,
            reply_id: json.reply_id,
            override_name: json.override_name,
        }
    };
    let message_id = data
        .message_create(DbMessageCreate {
            thread_id,
            attachment_ids: attachment_ids.clone(),
            author_id: user_id,
            message_type: MessageType::DefaultMarkdown(body.clone()),
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
    if let Some(content) = &body.content {
        for (ordering, link) in LinkFinder::new().links(content).enumerate() {
            if let Some(url) = link.as_str().parse::<Url>().ok() {
                let version_id = message.version_id;
                let s = s.clone();
                let srv = srv.clone();
                let data = s.data();
                tokio::spawn(async move {
                    let embed = srv.url_embed.generate(user_id, url).await?;
                    data.url_embed_link(version_id, embed.id, ordering as u32)
                        .await?;
                    let mut message = data.message_get(thread_id, message_id).await?;
                    s.presign_message(&mut message).await?;
                    s.broadcast_thread(
                        thread_id,
                        user_id,
                        None,
                        MessageSync::UpsertMessage { message },
                    )
                    .await?;
                    Result::Ok(())
                });
            }
        }
    }
    s.presign_message(&mut message).await?;
    message.nonce = nonce.or(json.nonce);
    data.thread_member_put(
        thread_id,
        user_id,
        ThreadMembership::Join {
            override_name: None,
            override_description: None,
        },
    )
    .await?;
    let msg = MessageSync::UpsertMessage {
        message: message.clone(),
    };
    srv.threads.invalidate(thread_id); // message count
    s.broadcast_thread(thread_id, user_id, reason, msg).await?;
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
    let before = data.message_list(thread_id, before_q).await?;
    let after_q = PaginationQuery {
        from: Some(message_id),
        to: q.to_end,
        dir: Some(PaginationDirection::F),
        limit: Some(limit),
    };
    let after = data.message_list(thread_id, after_q).await?;
    let message = data.message_get(thread_id, message_id).await?;
    let mut res = dbg!(ContextResponse {
        items: before
            .items
            .into_iter()
            .chain([message])
            .chain(after.items.into_iter())
            .collect(),
        total: after.total,
        has_after: after.has_more,
        has_before: before.has_more,
    });
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
    let mut res = data.message_list(thread_id, q).await?;
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
    let mut message = data.message_get(thread_id, message_id).await?;
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
    json.validate()?;
    let data = s.data();
    let mut perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let message = data.message_get(thread_id, message_id).await?;
    if !message.message_type.is_editable() {
        return Err(Error::BadStatic("cant edit that message"));
    }
    if message.author.id == user_id {
        perms.add(Permission::MessageEdit);
    }
    perms.ensure(Permission::MessageEdit)?;
    if json.content.is_none() && json.attachments.as_ref().is_some_and(|a| a.is_empty()) {
        return Err(Error::BadStatic(
            "at least one of content, attachments, or embeds must be defined",
        ));
    }
    if json.attachments.as_ref().is_none_or(|a| !a.is_empty()) {
        perms.ensure(Permission::MessageAttachments)?;
    }
    // if json.embeds.as_ref().is_none_or(|a| !a.is_empty()) {
    //     perms.ensure(Permission::MessageEmbeds)?;
    // }
    if !json.changes(&message) {
        return Ok((StatusCode::NOT_MODIFIED, Json(message)));
    }
    let attachment_ids: Vec<_> = json
        .attachments
        .map(|ats| ats.into_iter().map(|r| r.id).collect())
        .unwrap_or_else(|| match &message.message_type {
            MessageType::DefaultMarkdown(msg) => {
                msg.attachments.iter().map(|media| media.id).collect()
            }
            _ => vec![],
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
    let body = match message.message_type {
        MessageType::DefaultMarkdown(msg) => Result::Ok(MessageDefaultMarkdown {
            content: json.content.unwrap_or(msg.content),
            attachments: vec![],
            embeds: vec![],
            metadata: json.metadata.unwrap_or(msg.metadata),
            reply_id: json.reply_id.unwrap_or(msg.reply_id),
            override_name: json.override_name.unwrap_or(msg.override_name),
        }),
        _ => return Err(Error::Unimplemented),
    }?;
    let version_id = data
        .message_update(
            thread_id,
            message_id,
            DbMessageCreate {
                thread_id,
                attachment_ids: attachment_ids.clone(),
                author_id: user_id,
                message_type: MessageType::DefaultMarkdown(body.clone()),
            },
        )
        .await?;
    let version_uuid = version_id.into_inner();
    for id in &attachment_ids {
        data.media_link_insert(*id, version_uuid, MediaLinkType::MessageVersion)
            .await?;
    }
    let mut message = data.message_version_get(thread_id, version_id).await?;
    s.presign_message(&mut message).await?;
    s.broadcast_thread(
        thread_id,
        user_id,
        reason,
        MessageSync::UpsertMessage {
            message: message.clone(),
        },
    )
    .await?;
    s.services().threads.invalidate(thread_id); // last version id
    Ok((StatusCode::CREATED, Json(message)))
}

/// Delete message
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
    let message = data.message_get(thread_id, message_id).await?;
    if !message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete that message"));
    }
    if message.author.id == user_id {
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
        MessageSync::DeleteMessage {
            room_id: thread.room_id,
            thread_id,
            message_id,
        },
    )
    .await?;
    s.services().threads.invalidate(thread_id); // last version id, message count
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
    let mut res = data.message_version_list(thread_id, message_id, q).await?;
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
    let mut message = data.message_version_get(thread_id, version_id).await?;
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
    let message = data.message_version_get(thread_id, version_id).await?;
    if !message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete this message type"));
    }
    if message.author.id == user_id {
        perms.add(Permission::MessageDelete);
    }
    perms.ensure(Permission::MessageDelete)?;
    data.message_version_delete(thread_id, version_id).await?;
    s.services().threads.invalidate(thread_id); // last version id, message count
    Ok(Json(()))
}

/// Message delete bulk
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/messages/bulk-delete",
    params(
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = NO_CONTENT, description = "bulk delete success"),
    )
)]
async fn message_delete_bulk(
    Path(_thread_id): Path<ThreadId>,
    Auth(_user_id): Auth,
    HeaderReason(_reason): HeaderReason,
    State(_s): State<Arc<ServerState>>,
    Json(json): Json<MessageDeleteBulk>,
) -> Result<()> {
    json.validate()?;
    Err(Error::Unimplemented)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, IntoParams, Validate)]
struct MessageDeleteBulk {
    /// which messages to delete
    #[serde(default)]
    #[validate(length(min = 1, max = 128))]
    message_id: Vec<MessageId>,
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
}
