use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    tag::{Tag, TagCreate, TagPatch},
    util::Changes,
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, ChannelId, MessageSync, Permission, TagId,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Result,
    routes::util::{Auth, HeaderReason},
    Error, ServerState,
};

/// Create a tag
#[utoipa::path(
    post,
    path = "/channel/{channel_id}/tag",
    params(("channel_id", description = "The ID of the forum channel to create the tag in.")),
    tags = ["tag", "badge.perm.TagManage"],
    responses(
        (status = CREATED, body = Tag, description = "Create tag success"),
    )
)]
pub async fn tag_create(
    Path(channel_id): Path<ChannelId>,
    State(s): State<Arc<ServerState>>,
    Auth(user): Auth,
    HeaderReason(reason): HeaderReason,
    Json(create): Json<TagCreate>,
) -> Result<impl IntoResponse> {
    user.ensure_unsuspended()?;
    create.validate()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(user.id, channel_id).await?;
    perms.ensure(Permission::TagManage)?;

    let channel = srv.channels.get(channel_id, Some(user.id)).await?;
    if !channel.ty.has_tags() {
        return Err(Error::BadStatic("channel does not support tags"));
    }

    let chan_old = srv.channels.get(channel_id, Some(user.id)).await?;
    let tag = s.data().tag_create(channel_id, create).await?;

    srv.channels.invalidate(channel_id).await;
    let chan_new = srv.channels.get(channel_id, Some(user.id)).await?;

    if let Some(room_id) = chan_new.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: user.id,
            session_id: None,
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
        user.id,
        MessageSync::ChannelUpdate {
            channel: Box::new(chan_new),
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(tag)))
}

/// Update a tag
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
pub async fn tag_update(
    Path((channel_id, tag_id)): Path<(ChannelId, TagId)>,
    State(s): State<Arc<ServerState>>,
    Auth(user): Auth,
    HeaderReason(reason): HeaderReason,
    Json(patch): Json<TagPatch>,
) -> Result<impl IntoResponse> {
    user.ensure_unsuspended()?;
    patch.validate()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(user.id, channel_id).await?;
    perms.ensure(Permission::TagManage)?;

    let tag_channel_id = s.data().tag_get_forum_id(tag_id).await?;
    if channel_id != tag_channel_id {
        return Err(Error::NotFound);
    }

    let chan_old = srv.channels.get(channel_id, Some(user.id)).await?;
    let tag = s.data().tag_update(tag_id, patch).await?;

    srv.channels.invalidate(channel_id).await;
    let chan_new = srv.channels.get(channel_id, Some(user.id)).await?;

    if let Some(room_id) = chan_new.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: user.id,
            session_id: None,
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
        user.id,
        MessageSync::ChannelUpdate {
            channel: Box::new(chan_new),
        },
    )
    .await?;

    Ok((StatusCode::OK, Json(tag)))
}

/// Delete a tag
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
pub async fn tag_delete(
    Path((channel_id, tag_id)): Path<(ChannelId, TagId)>,
    State(s): State<Arc<ServerState>>,
    Auth(user): Auth,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(user.id, channel_id).await?;
    perms.ensure(Permission::TagManage)?;

    let tag_channel_id = s.data().tag_get_forum_id(tag_id).await?;
    if channel_id != tag_channel_id {
        return Err(Error::NotFound);
    }

    let chan_old = srv.channels.get(channel_id, Some(user.id)).await?;
    s.data().tag_delete(tag_id).await?;

    srv.channels.invalidate(channel_id).await;
    let chan_new = srv.channels.get(channel_id, Some(user.id)).await?;

    if let Some(room_id) = chan_new.room_id {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: user.id,
            session_id: None,
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
        user.id,
        MessageSync::ChannelUpdate {
            channel: Box::new(chan_new),
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(tag_create))
        .routes(routes!(tag_update))
        .routes(routes!(tag_delete))
}
