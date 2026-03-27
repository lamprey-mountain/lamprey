use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::util::Changes;
use common::v1::types::{AuditLogEntryType, MessageSync, Permission};
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;
use validator::Validate;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Tag create
#[handler(routes::tag_create)]
async fn tag_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::tag_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    req.tag.validate()?;

    let srv = s.services();
    let tag = srv
        .tag
        .create(req.channel_id, &auth, req.tag, req.idempotency_key)
        .await?;

    Ok((StatusCode::CREATED, Json(tag)))
}

/// Tag update
#[handler(routes::tag_update)]
async fn tag_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::tag_update::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    req.patch.validate()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelEdit)?;

    let tag_channel_id = s.data().tag_get_forum_id(req.tag_id).await?;
    if req.channel_id != tag_channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownTag).into());
    }

    let tag_old = s.data().tag_get(req.tag_id).await?;
    let tag = s.data().tag_update(req.tag_id, req.patch).await?;

    let channel = srv
        .channels
        .get(tag_old.channel_id, Some(auth.user.id))
        .await?;
    if let Some(room_id) = channel.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::TagUpdate {
            channel_id: req.channel_id,
            tag_id: req.tag_id,
            changes: Changes::new()
                .change("name", &tag_old.name, &tag.name)
                .change("description", &tag_old.description, &tag.description)
                .change("color", &tag_old.color, &tag.color)
                .change("archived", &tag_old.archived, &tag.archived)
                .change("restricted", &tag_old.restricted, &tag.restricted)
                .build(),
        })
        .await?;
    }

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::TagUpdate { tag: tag.clone() },
    )
    .await?;

    Ok((StatusCode::OK, Json(tag)))
}

/// Tag delete
#[handler(routes::tag_delete)]
async fn tag_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::tag_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelEdit)?;

    let tag_channel_id = s.data().tag_get_forum_id(req.tag_id).await?;
    if req.channel_id != tag_channel_id {
        return Err(ApiError::from_code(ErrorCode::UnknownTag).into());
    }

    let tag = s.data().tag_get(req.tag_id).await?;

    if tag.total_thread_count > 0 && !req.query.force {
        return Ok(StatusCode::CONFLICT.into_response());
    }

    s.data().tag_delete(req.tag_id).await?;

    let channel = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    if let Some(room_id) = channel.room_id {
        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::TagDelete {
            channel_id: req.channel_id,
            tag_id: req.tag_id,
            changes: Changes::new()
                .remove("name", &tag.name)
                .remove("description", &tag.description)
                .build(),
        })
        .await?;
    }

    s.broadcast_channel(
        req.channel_id,
        auth.user.id,
        MessageSync::TagDelete {
            tag_id: req.tag_id,
            channel_id: req.channel_id,
        },
    )
    .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Tag list
#[handler(routes::tag_list)]
async fn tag_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::tag_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;

    let tags = s
        .data()
        .tag_list(req.channel_id, req.list.archived, req.pagination)
        .await?;
    Ok(Json(tags))
}

/// Tag get
#[handler(routes::tag_get)]
async fn tag_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::tag_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;

    let tag = s.data().tag_get(req.tag_id).await?;
    Ok(Json(tag))
}

/// Tag search
#[handler(routes::tag_search)]
async fn tag_search(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::tag_search::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let perms = srv.perms.for_channel(auth.user.id, req.channel_id).await?;
    perms.ensure(Permission::ChannelView)?;

    let tags = s
        .data()
        .tag_search(
            req.channel_id,
            req.search.query,
            req.search.archived,
            req.pagination,
        )
        .await?;
    Ok(Json(tags))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(tag_create))
        .routes(routes2!(tag_update))
        .routes(routes2!(tag_delete))
        .routes(routes2!(tag_list))
        .routes(routes2!(tag_get))
        .routes(routes2!(tag_search))
}
