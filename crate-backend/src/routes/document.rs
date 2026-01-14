use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use common::v1::types::ids::DocumentBranchId;
use common::v1::types::{
    document::{BranchCreate, BranchMerge, BranchPatch},
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
    _json: Json<BranchPatch>,
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

/// Document branch create (TODO)
#[utoipa::path(
    post,
    path = "/document/{channel_id}/branch",
    params(("channel_id", description = "Channel id")),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_branch_create(
    Path(_channel_id): Path<ChannelId>,
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _json: Json<BranchCreate>,
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
    _json: Json<BranchMerge>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(document_branch_list))
        .routes(routes!(document_branch_get))
        .routes(routes!(document_branch_update))
        .routes(routes!(document_branch_delete))
        .routes(routes!(document_branch_create))
        .routes(routes!(document_branch_merge))
}
