use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    document::{
        DocumentBranch, DocumentBranchCreate, DocumentBranchListParams, DocumentBranchMerge,
        DocumentBranchPatch, DocumentRevisionId, DocumentTagCreate, DocumentTagPatch,
    },
    pagination::{PaginationQuery, PaginationResponse},
    ChannelId, Permission,
};
use common::v1::types::{
    document::{HistoryPagination, HistoryParams},
    ids::{DocumentBranchId, DocumentTagId},
    MessageSync,
};

use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;

use crate::error::Result;
use crate::{Error, ServerState};

#[allow(unused)] // TEMP
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct BranchDeleteQuery {
    #[serde(default)]
    pub force: bool,
}

/// Wiki history (TODO)
#[utoipa::path(
    get,
    path = "/wiki/{channel_id}/history",
    params(
        ("channel_id", description = "Channel id"),
        HistoryParams
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok", body = HistoryPagination),
    )
)]
async fn wiki_history(
    Path(_channel_id): Path<ChannelId>,
    Query(_query): Query<HistoryParams>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Document branch list
#[utoipa::path(
    get,
    path = "/document/{channel_id}/branch",
    params(
        ("channel_id", description = "Channel id"),
        DocumentBranchListParams,
        PaginationQuery<DocumentBranchId>
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok", body = PaginationResponse<DocumentBranch>),
    )
)]
async fn document_branch_list(
    Path(channel_id): Path<ChannelId>,
    Query(query): Query<DocumentBranchListParams>,
    Query(pagination): Query<PaginationQuery<DocumentBranchId>>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let branches = data
        .document_branch_paginate(channel_id, query, pagination)
        .await?;

    Ok(Json(branches))
}

/// Document branch get (TODO)
#[utoipa::path(
    get,
    path = "/document/{channel_id}/branch/{branch_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_branch_get(
    Path((_channel_id, _branch_id)): Path<(ChannelId, DocumentBranchId)>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Document branch update (TODO)
#[utoipa::path(
    patch,
    path = "/document/{channel_id}/branch/{branch_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_branch_update(
    Path((_channel_id, _branch_id)): Path<(ChannelId, DocumentBranchId)>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _json: Json<DocumentBranchPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Document branch delete (TODO)
#[utoipa::path(
    delete,
    path = "/document/{channel_id}/branch/{branch_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
        BranchDeleteQuery,
    ),
    tags = ["document"],
    responses(
        (status = NO_CONTENT, description = "ok"),
    )
)]
async fn document_branch_delete(
    Path((_channel_id, _branch_id)): Path<(ChannelId, DocumentBranchId)>,
    Query(_query): Query<BranchDeleteQuery>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Document branch fork (TODO)
#[utoipa::path(
    post,
    path = "/document/{channel_id}/branch/{parent_id}/fork",
    params(("channel_id", description = "Channel id"), ("parent_id", description = "Parent branch id")),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_branch_fork(
    Path(_channel_id): Path<ChannelId>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _json: Json<DocumentBranchCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Document branch merge (TODO)
#[utoipa::path(
    post,
    path = "/document/{channel_id}/branch/{branch_id}/merge",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_branch_merge(
    Path((_channel_id, _branch_id)): Path<(ChannelId, DocumentBranchId)>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _json: Json<DocumentBranchMerge>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Document tag create
#[utoipa::path(
    post,
    path = "/document/{channel_id}/tag",
    params(("channel_id", description = "Channel id")),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_tag_create(
    Path(channel_id): Path<ChannelId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<DocumentTagCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let user_id = auth.user.id;
    let data = s.data();
    let srv = s.services();

    let perms = srv.perms.for_channel(user_id, channel_id).await?;
    perms.ensure(Permission::DocumentEdit)?;

    let DocumentTagCreate {
        summary,
        description,
        revision,
    } = json;

    let (branch_id, revision_seq) = match revision {
        DocumentRevisionId::Branch { branch_id: _ } => {
            // TODO: implement tagging branch heads
            return Err(Error::Unimplemented);
        }
        DocumentRevisionId::Revision { branch_id, seq } => (branch_id, seq),
        DocumentRevisionId::Tag { .. } => {
            return Err(Error::BadRequest("Cannot tag another tag".to_string()));
        }
    };

    let tag_id = data
        .document_tag_create(branch_id, user_id, summary, description, revision_seq)
        .await?;

    let tag = data.document_tag_get(tag_id).await?;

    s.broadcast(MessageSync::DocumentTagCreate {
        channel_id,
        tag: tag.clone(),
    })?;

    Ok(Json(tag))
}

/// Document tag list
#[utoipa::path(
    get,
    path = "/document/{channel_id}/tag",
    params(("channel_id", description = "Channel id")),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_tag_list(
    Path(channel_id): Path<ChannelId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let user_id = auth.user.id;
    let data = s.data();
    let srv = s.services();

    let perms = srv.perms.for_channel(user_id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let tags = data.document_tag_list_by_document(channel_id).await?;
    Ok(Json(tags))
}

/// Document tag get
#[utoipa::path(
    get,
    path = "/document/{channel_id}/tag/{tag_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("tag_id", description = "Tag id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_tag_get(
    Path((channel_id, tag_id)): Path<(ChannelId, DocumentTagId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let user_id = auth.user.id;
    let srv = s.services();

    let perms = srv.perms.for_channel(user_id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let data = s.data();
    let tag = data.document_tag_get(tag_id).await?;
    Ok(Json(tag))
}

/// Document tag update
#[utoipa::path(
    patch,
    path = "/document/{channel_id}/tag/{tag_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("tag_id", description = "Tag id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_tag_update(
    Path((channel_id, tag_id)): Path<(ChannelId, DocumentTagId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    json: Json<DocumentTagPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let user_id = auth.user.id;
    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_channel(user_id, channel_id).await?;
    perms.ensure(Permission::DocumentEdit)?;

    let tag = data.document_tag_get(tag_id).await?;

    if tag.creator_id != Some(user_id) {
        perms.ensure(Permission::ThreadManage)?;
    }

    let DocumentTagPatch {
        summary,
        description,
    } = json.0;

    data.document_tag_update(tag_id, summary, description)
        .await?;

    let updated_tag = data.document_tag_get(tag_id).await?;

    s.broadcast(MessageSync::DocumentTagUpdate {
        channel_id,
        tag: updated_tag.clone(),
    })?;

    Ok(Json(updated_tag))
}

/// Document tag delete
#[utoipa::path(
    delete,
    path = "/document/{channel_id}/tag/{tag_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("tag_id", description = "Tag id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_tag_delete(
    Path((channel_id, tag_id)): Path<(ChannelId, DocumentTagId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let user_id = auth.user.id;
    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_channel(user_id, channel_id).await?;
    perms.ensure(Permission::DocumentEdit)?;

    let tag = data.document_tag_get(tag_id).await?;

    if tag.creator_id != Some(user_id) {
        perms.ensure(Permission::ThreadManage)?;
    }

    let branch_id = tag.branch_id;

    data.document_tag_delete(tag_id).await?;

    s.broadcast(MessageSync::DocumentTagDelete {
        channel_id,
        branch_id,
        tag_id,
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Document history (TODO)
#[utoipa::path(
    get,
    path = "/document/{channel_id}/branch/{branch_id}/history",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
        HistoryParams
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok", body = HistoryPagination),
    )
)]
async fn document_history(
    Path((_channel_id, _branch_id)): Path<(ChannelId, DocumentBranchId)>,
    Query(_query): Query<HistoryParams>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(wiki_history))
        .routes(routes!(document_branch_list))
        .routes(routes!(document_branch_get))
        .routes(routes!(document_branch_update))
        .routes(routes!(document_branch_delete))
        .routes(routes!(document_branch_fork))
        .routes(routes!(document_branch_merge))
        .routes(routes!(document_tag_create))
        .routes(routes!(document_tag_list))
        .routes(routes!(document_tag_get))
        .routes(routes!(document_tag_update))
        .routes(routes!(document_tag_delete))
        .routes(routes!(document_history))
}
