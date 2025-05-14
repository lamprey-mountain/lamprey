use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    misc::Color, reaction::ReactionCounts, util::Diff, Embed, EmbedId, Interactions,
    MessageDefaultMarkdown, MessageDefaultTagged, PaginationDirection, ThreadMembership,
};
use linkify::LinkFinder;
use serde::{Deserialize, Serialize};
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
    if !json.embeds.is_empty() {
        perms.ensure(Permission::MessageEmbeds)?;
    }
    // TODO: move this to validation
    if json.content.as_ref().is_none_or(|s| s.is_empty())
        && json.attachments.is_empty()
        && json.embeds.is_empty()
    {
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
    let (content, payload) = if json.use_new_text_formatting {
        (
            json.content.clone(),
            MessageType::DefaultTagged(MessageDefaultTagged {
                content: json.content,
                attachments: vec![],
                embeds: vec![],
                metadata: json.metadata,
                reply_id: json.reply_id,
                reactions: ReactionCounts(vec![]),
                interactions: Interactions::default(),
            }),
        )
    } else {
        (
            json.content.clone(),
            MessageType::DefaultMarkdown(MessageDefaultMarkdown {
                content: json.content,
                attachments: vec![],
                embeds: vec![],
                metadata: json.metadata,
                reply_id: json.reply_id,
                override_name: json.override_name,
                reactions: ReactionCounts::default(),
            }),
        )
    };
    let message_id = data
        .message_create(DbMessageCreate {
            thread_id,
            attachment_ids: attachment_ids.clone(),
            author_id: user_id,
            message_type: payload,
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
    for (ordering, embed) in json.embeds.iter().enumerate() {
        let id = EmbedId::new();
        let embed = Embed {
            id,
            url: embed.url.clone(),
            canonical_url: None,
            title: embed.title.clone(),
            description: embed.description.clone(),
            color: embed
                .color
                .clone()
                .map(|s| csscolorparser::parse(&s))
                .transpose()
                .map_err(|e| Error::GenericError(e.to_string()))?
                .map(|c| Color::from_hex_string(c.to_hex_string())),
            media: None,
            media_is_thumbnail: embed.media_is_thumbnail,
            author_name: embed.author_name.clone(),
            author_url: embed.author_url.clone(),
            author_avatar: None,
            site_name: None,
            site_avatar: None,
        }
        .truncate();
        data.embed_insert(user_id, embed.clone()).await?;
        data.embed_link(message.version_id, id, ordering as u32)
            .await?;
        match &mut message.message_type {
            MessageType::DefaultMarkdown(m) => m.embeds.push(embed),
            MessageType::DefaultTagged(m) => m.embeds.push(embed),
            _ => {}
        }
    }

    if let Some(content) = &content {
        for (ordering, link) in LinkFinder::new().links(content).enumerate() {
            if let Some(url) = link.as_str().parse::<Url>().ok() {
                let version_id = message.version_id;
                let s = s.clone();
                let srv = srv.clone();
                let data = s.data();
                let embed_count = json.embeds.len();
                tokio::spawn(async move {
                    let embed = srv.embed.generate(user_id, url).await?;
                    data.embed_link(version_id, embed.id, (ordering + embed_count) as u32)
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
    if message.author_id == user_id {
        perms.add(Permission::MessageEdit);
    }
    perms.ensure(Permission::MessageEdit)?;
    if json.content.is_none()
        && json.attachments.as_ref().is_some_and(|a| a.is_empty())
        && json.embeds.as_ref().is_some_and(|a| a.is_empty())
    {
        return Err(Error::BadStatic(
            "at least one of content, attachments, or embeds must be defined",
        ));
    }
    if json.attachments.as_ref().is_none_or(|a| !a.is_empty()) {
        perms.ensure(Permission::MessageAttachments)?;
    }
    if json.embeds.as_ref().is_none_or(|a| !a.is_empty()) {
        perms.ensure(Permission::MessageEmbeds)?;
    }
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
    let (content, payload) = match message.message_type.clone() {
        MessageType::DefaultMarkdown(msg) => {
            let content = json.content.unwrap_or(msg.content);
            Result::Ok((
                content.clone(),
                MessageType::DefaultMarkdown(MessageDefaultMarkdown {
                    content,
                    attachments: vec![],
                    embeds: vec![],
                    metadata: json.metadata.unwrap_or(msg.metadata),
                    reply_id: json.reply_id.unwrap_or(msg.reply_id),
                    override_name: json.override_name.unwrap_or(msg.override_name),
                    reactions: ReactionCounts::default(),
                }),
            ))
        }
        MessageType::DefaultTagged(msg) => {
            let content = json.content.unwrap_or(msg.content);
            Result::Ok((
                content.clone(),
                MessageType::DefaultTagged(MessageDefaultTagged {
                    content,
                    attachments: vec![],
                    embeds: vec![],
                    metadata: json.metadata.unwrap_or(msg.metadata),
                    reply_id: json.reply_id.unwrap_or(msg.reply_id),
                    reactions: ReactionCounts(vec![]),
                    interactions: Interactions::default(),
                }),
            ))
        }
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
                message_type: payload,
            },
        )
        .await?;

    let version_uuid = version_id.into_inner();
    for id in &attachment_ids {
        data.media_link_insert(*id, version_uuid, MediaLinkType::MessageVersion)
            .await?;
    }

    if let Some(content) = &content {
        let embeds = match &message.message_type {
            MessageType::DefaultMarkdown(m) => Some(m.embeds.clone()),
            MessageType::DefaultTagged(m) => Some(m.embeds.clone()),
            _ => None,
        };
        if let Some(embeds) = dbg!(embeds) {
            let srv = s.services();
            for (ordering, link) in LinkFinder::new().links(content).enumerate() {
                if let Some(url) = link.as_str().parse::<Url>().ok() {
                    let s = s.clone();
                    let srv = srv.clone();
                    let data = s.data();
                    let embeds = embeds.clone();
                    tokio::spawn(async move {
                        let embed = if let Some(existing) = embeds
                            .iter()
                            .find(|e| e.url.as_ref().is_some_and(|u| u == &url))
                        {
                            existing.clone()
                        } else {
                            srv.embed.generate(user_id, url).await?
                        };
                        data.embed_link(version_id, embed.id, ordering as u32)
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
    }

    let mut message = data.message_version_get(thread_id, version_id).await?;

    if let Some(embeds) = json.embeds {
        for (ordering, embed) in embeds.iter().enumerate() {
            // TODO: don't create new embeds for every version
            // TODO: don't always create a new version if any embeds for every version
            let id = EmbedId::new();
            let embed = Embed {
                id,
                url: embed.url.clone(),
                canonical_url: None,
                title: embed.title.clone(),
                description: embed.description.clone(),
                color: embed
                    .color
                    .clone()
                    .map(|s| csscolorparser::parse(&s))
                    .transpose()
                    .map_err(|e| Error::GenericError(e.to_string()))?
                    .map(|c| Color::from_hex_string(c.to_hex_string())),
                media: None,
                media_is_thumbnail: embed.media_is_thumbnail,
                author_name: embed.author_name.clone(),
                author_url: embed.author_url.clone(),
                author_avatar: None,
                site_name: None,
                site_avatar: None,
            }
            .truncate();
            data.embed_insert(user_id, embed.clone()).await?;
            data.embed_link(message.version_id, id, ordering as u32)
                .await?;
            match &mut message.message_type {
                MessageType::DefaultMarkdown(m) => m.embeds.push(embed),
                MessageType::DefaultTagged(m) => m.embeds.push(embed),
                _ => {}
            }
        }
    }

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
    if message.author_id == user_id {
        perms.add(Permission::MessageDelete);
    }
    perms.ensure(Permission::MessageDelete)?;
    data.message_version_delete(thread_id, version_id).await?;
    s.services().threads.invalidate(thread_id); // last version id, message count
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
struct MessageBulkUndelete {
    /// which messages to undelete
    #[serde(default)]
    #[validate(length(min = 1, max = 128))]
    message_ids: Vec<MessageId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, Validate)]
struct MessageBulkMove {
    /// which messages to move
    #[serde(default)]
    #[validate(length(min = 1, max = 128))]
    message_ids: Vec<MessageId>,

    /// keep original messages intact
    copy: bool,

    /// must be in same room (for now...)
    target_thread_id: ThreadId,
}

/// Message delete bulk (TODO)
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/messages/delete",
    params(("thread_id", description = "Thread id")),
    tags = ["message"],
    responses((status = NO_CONTENT, description = "bulk delete success")),
)]
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
        let message = data.message_get(thread_id, *id).await?;
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
    s.services().threads.invalidate(thread_id); // last version id, message count
    Ok(StatusCode::NO_CONTENT)
}

/// Message undelete (TODO)
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/messages/undelete",
    params(("thread_id", description = "Thread id")),
    tags = ["message"],
    responses((status = NO_CONTENT, description = "undelete success")),
)]
async fn message_undelete(
    Path(_thread_id): Path<ThreadId>,
    Auth(_user_id): Auth,
    HeaderReason(_reason): HeaderReason,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<MessageBulkUndelete>,
) -> Result<()> {
    todo!()
}

/// Message move (TODO)
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/messages/move",
    params(("thread_id", description = "Thread id")),
    tags = ["message"],
    responses((status = NO_CONTENT, description = "move success")),
)]
async fn message_move(
    Path(_thread_id): Path<ThreadId>,
    Auth(_user_id): Auth,
    HeaderReason(_reason): HeaderReason,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<MessageBulkMove>,
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
}

/// always returns one
fn fn_one() -> u16 {
    1
}

/// Message replies (TODO)
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
    Path((_thread_id, _message_id)): Path<(ThreadId, MessageId)>,
    Query(_q): Query<RepliesQuery>,
    Auth(_user_id): Auth,
    HeaderReason(_reason): HeaderReason,
    State(_s): State<Arc<ServerState>>,
) -> Result<()> {
    Err(Error::Unimplemented)
}

// /// Message append (TODO)
// ///
// /// A restricted variant of message edit
// ///
// /// - Only allows appending to `content`
// /// - Message version must be less than 5 minutes old
// /// - Message will not get a new version
// /// - Intended for dynamic/streaming responses
// ///
// /// maybe see InteractionStatus
// #[utoipa::path(
//     patch,
//     path = "/thread/{thread_id}/message/{message_id}/append",
//     params(
//         RepliesQuery,
//         ("thread_id", description = "Thread id"),
//         ("message_id", description = "Message id"),
//     ),
//     tags = ["message"],
//     responses(
//         (status = OK, body = Message, description = "success"),
//         (status = NOT_MODIFIED, description = "Not modified"),
//     ),
// )]
// async fn message_append(
//     Path((_thread_id, _message_id)): Path<(ThreadId, MessageId)>,
//     Auth(_user_id): Auth,
//     State(_s): State<Arc<ServerState>>,
//     Json(_json): Json<MessagePatch>,
// ) -> Result<()> {
//     // json.can_append(other)
//     Err(Error::Unimplemented)
// }

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
        .routes(routes!(message_undelete))
        .routes(routes!(message_move))
}
