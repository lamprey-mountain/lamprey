use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    tag::{Tag, TagCreate, TagDeleteQuery, TagListQuery, TagPatch, TagSearchQuery},
    util::Changes,
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, ChannelId, MessageSync, PaginationQuery,
    PaginationResponse, Permission, TagId,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Result,
    routes::util::{Auth2, HeaderReason},
    Error, ServerState,
};

/// Tag create
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/tag",
    params(("channel_id", description = "The ID of the forum channel to create the tag in.")),
    tags = ["tag", "badge.perm.TagManage"],
    responses(
        (status = CREATED, body = Tag, description = "Create tag success"),
    )
)]
async fn tag_create(
    Path(channel_id): Path<ChannelId>,
    State(s): State<Arc<ServerState>>,
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    Json(create): Json<TagCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    create.validate()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::TagManage)?;

    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !channel.ty.has_tags() {
        return Err(Error::BadStatic("channel does not support tags"));
    }

    let chan_old = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    let tag = s.data().tag_create(channel_id, create).await?;

    srv.channels.invalidate(channel_id).await;
    let chan_new = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = chan_new.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason: reason.clone(), // No reason header here
            ty: AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type: chan_new.ty,
                changes: Changes::new()
                    .change(
                        "tags_available",
                        &chan_old.tags_available,
                        &chan_new.tags_available,
                    )
                    .build(),
            },
        })
        .await?;
    }

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::ChannelUpdate {
            channel: Box::new(chan_new),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(tag)))
}

/// Tag update
#[utoipa::path(
    patch,
    path = "/channel/{channel_id}/tag/{tag_id}",
    params(
        ("channel_id", description = "The ID of the forum channel the tag belongs to."),
        ("tag_id", description = "The ID of the tag to update.")
    ),
    tags = ["tag", "badge.perm.TagManage"],
    responses(
        (status = OK, body = Tag, description = "Update tag success"),
    )
)]
async fn tag_update(
    Path((channel_id, tag_id)): Path<(ChannelId, TagId)>,
    State(s): State<Arc<ServerState>>,
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
    Json(patch): Json<TagPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    patch.validate()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::TagManage)?;

    let tag_channel_id = s.data().tag_get_forum_id(tag_id).await?;
    if channel_id != tag_channel_id {
        return Err(Error::NotFound);
    }

    let chan_old = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    let tag = s.data().tag_update(tag_id, patch).await?;

    srv.channels.invalidate(channel_id).await;
    let chan_new = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = chan_new.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason,
            ty: AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type: chan_new.ty,
                changes: Changes::new()
                    .change(
                        "tags_available",
                        &chan_old.tags_available,
                        &chan_new.tags_available,
                    )
                    .build(),
            },
        })
        .await?;
    }

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::ChannelUpdate {
            channel: Box::new(chan_new),
        },
    )
    .await?;

    Ok((StatusCode::OK, Json(tag)))
}

/// Tag delete
#[utoipa::path(
    delete,
    path = "/channel/{channel_id}/tag/{tag_id}",
    params(
        ("channel_id", description = "The ID of the forum channel the tag belongs to."),
        ("tag_id", description = "The ID of the tag to delete.")
    ),
    tags = ["tag", "badge.perm.TagManage"],
    responses(
        (status = NO_CONTENT, description = "Delete tag success"),
    )
)]
async fn tag_delete(
    Path((channel_id, tag_id)): Path<(ChannelId, TagId)>,
    Query(query): Query<TagDeleteQuery>,
    State(s): State<Arc<ServerState>>,
    auth: Auth2,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::TagManage)?;

    let tag_channel_id = s.data().tag_get_forum_id(tag_id).await?;
    if channel_id != tag_channel_id {
        return Err(Error::NotFound);
    }

    let tag = s.data().tag_get(tag_id).await?;

    if tag.total_thread_count > 0 && !query.force {
        return Ok(StatusCode::CONFLICT.into_response());
    }

    let chan_old = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    s.data().tag_delete(tag_id).await?;

    srv.channels.invalidate(channel_id).await;
    let chan_new = srv.channels.get(channel_id, Some(auth.user.id)).await?;

    if let Some(room_id) = chan_new.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason,
            ty: AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type: chan_new.ty,
                changes: Changes::new()
                    .change(
                        "tags_available",
                        &chan_old.tags_available,
                        &chan_new.tags_available,
                    )
                    .build(),
            },
        })
        .await?;
    }

    s.broadcast_channel(
        channel_id,
        auth.user.id,
        MessageSync::ChannelUpdate {
            channel: Box::new(chan_new),
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Tag search
///
/// Search for tags in a forum channel.
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/tag/search",
    params(
        ("channel_id", description = "The ID of the forum channel to search for tags in."),
        TagSearchQuery,
        PaginationQuery<TagId>,
    ),
    tags = ["tag"],
    responses(
        (status = OK, body = PaginationResponse<Tag>, description = "success"),
    )
)]
async fn tag_search(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    Query(q): Query<TagSearchQuery>,
    Query(pagination): Query<PaginationQuery<TagId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !channel.ty.has_tags() {
        return Err(Error::BadStatic("channel does not support tags"));
    }

    let tags = s
        .data()
        .tag_search(channel_id, q.query, q.archived, pagination)
        .await?;

    Ok(Json(tags))
}

/// Tag list
///
/// List all tags in a forum channel.
#[utoipa::path(
    get,
    path = "/channel/{channel_id}/tag",
    params(
        ("channel_id", description = "The ID of the forum channel to list tags from."),
        TagListQuery,
        PaginationQuery<TagId>,
    ),
    tags = ["tag"],
    responses(
        (status = OK, body = PaginationResponse<Tag>, description = "success"),
    )
)]
async fn tag_list(
    Path(channel_id): Path<ChannelId>,
    auth: Auth2,
    Query(q): Query<TagListQuery>,
    Query(pagination): Query<PaginationQuery<TagId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    if !channel.ty.has_tags() {
        return Err(Error::BadStatic("channel does not support tags"));
    }

    let tags = s.data().tag_list(channel_id, q.archived, pagination).await?;

    Ok(Json(tags))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(tag_create))
        .routes(routes!(tag_update))
        .routes(routes!(tag_delete))
        .routes(routes!(tag_search))
        .routes(routes!(tag_list))
}
