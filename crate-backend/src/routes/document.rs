use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::ids::{DocumentBranchId, DocumentTagId};
use common::v1::types::pagination::{HistoryPagination, HistoryParams};
use common::v1::types::{
    document::{
        DocumentBranchCreate, DocumentBranchMerge, DocumentBranchPatch, DocumentTagCreate,
        DocumentTagPatch,
    },
    ChannelId,
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

/// Document branch list (TODO)
#[utoipa::path(
    get,
    path = "/document/{channel_id}/branch",
    params(("channel_id", description = "Channel id")),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_branch_list(
    Path(_channel_id): Path<ChannelId>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
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

/// Document tag create (TODO)
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
    Path(_channel_id): Path<ChannelId>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _json: Json<DocumentTagCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Document tag list (TODO)
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
    Path(_channel_id): Path<ChannelId>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Document tag get (TODO)
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
    Path((_channel_id, _tag_id)): Path<(ChannelId, DocumentTagId)>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Document tag update (TODO)
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
    Path((_channel_id, _tag_id)): Path<(ChannelId, DocumentTagId)>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _json: Json<DocumentTagPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

/// Document tag delete (TODO)
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
    Path((_channel_id, _tag_id)): Path<(ChannelId, DocumentTagId)>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
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
