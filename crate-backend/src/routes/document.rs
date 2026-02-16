use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::v1::types::{
    document::serialized::Serdoc,
    document::{
        DocumentBranch, DocumentBranchCreate, DocumentBranchListParams, DocumentBranchMerge,
        DocumentBranchPatch, DocumentBranchState, DocumentCrdtDiffParams, DocumentRevisionId,
        DocumentStateVector, DocumentTagCreate, DocumentTagPatch, SerdocPut,
    },
    pagination::{PaginationQuery, PaginationResponse},
    ChannelId, Permission,
};
use common::v1::types::{
    document::{HistoryPagination, HistoryParams},
    ids::{DocumentBranchId, DocumentTagId},
    MessageSync,
};
use uuid::Uuid;

use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;

use crate::error::Result;
use crate::{Error, ServerState};

// TODO: check if channels are actually wikis/documents

/// Wiki history
///
/// query edit history for all documents in this wiki
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
    Path(channel_id): Path<ChannelId>,
    Query(query): Query<HistoryParams>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let summary = srv.documents.query_wiki_history(channel_id, query).await?;

    let user_ids = summary.user_ids();
    let users = srv.users.get_many(&user_ids).await?;

    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    let room_members = if let Some(room_id) = channel.room_id {
        data.room_member_get_many(room_id, &user_ids).await?
    } else {
        vec![]
    };

    let thread_members = data.thread_member_get_many(channel_id, &user_ids).await?;

    Ok(Json(HistoryPagination {
        changesets: summary.changesets,
        users,
        room_members,
        thread_members,
        document_tags: summary.tags,
    }))
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
    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let branches = data
        .document_branch_paginate(channel_id, auth.user.id, query, pagination)
        .await?;

    Ok(Json(branches))
}

/// Document branch get
#[utoipa::path(
    get,
    path = "/document/{channel_id}/branch/{branch_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok", body = DocumentBranch),
    )
)]
async fn document_branch_get(
    Path((channel_id, branch_id)): Path<(ChannelId, DocumentBranchId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;

    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::NotFound);
    }

    Ok(Json(branch))
}

/// Document branch update
#[utoipa::path(
    patch,
    path = "/document/{channel_id}/branch/{branch_id}",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok", body = DocumentBranch),
    )
)]
async fn document_branch_update(
    Path((channel_id, branch_id)): Path<(ChannelId, DocumentBranchId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<DocumentBranchPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::DocumentEdit)?;

    let branch_before = data.document_branch_get(channel_id, branch_id).await?;

    if branch_before.creator_id != auth.user.id {
        if branch_before.private {
            return Err(Error::NotFound);
        }
        perms.ensure(Permission::ThreadManage)?;
    }

    data.document_branch_update(channel_id, branch_id, json)
        .await?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;

    s.broadcast(MessageSync::DocumentBranchUpdate {
        branch: branch.clone(),
    })?;

    Ok(Json(branch))
}

/// Document branch close
#[utoipa::path(
    post,
    path = "/document/{channel_id}/branch/{branch_id}/close",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok", body = DocumentBranch),
    )
)]
async fn document_branch_close(
    Path((channel_id, branch_id)): Path<(ChannelId, DocumentBranchId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::DocumentEdit)?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;

    if branch.default {
        return Err(Error::BadRequest("cannot close default branch".to_string()));
    }

    if branch.creator_id != auth.user.id {
        if branch.private {
            return Err(Error::NotFound);
        }
        perms.ensure(Permission::ThreadManage)?;
    }

    data.document_branch_set_state(channel_id, branch_id, DocumentBranchState::Closed)
        .await?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;

    s.broadcast(MessageSync::DocumentBranchDelete {
        channel_id,
        branch_id,
    })?;

    Ok(Json(branch))
}

/// Document branch fork
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
    Path((channel_id, parent_id)): Path<(ChannelId, DocumentBranchId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<DocumentBranchCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();
    let user_id = auth.user.id;

    let perms = srv.perms.for_channel(user_id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;
    perms.ensure(Permission::DocumentEdit)?;

    let parent_branch = data.document_branch_get(channel_id, parent_id).await?;
    if parent_branch.private && parent_branch.creator_id != user_id {
        return Err(Error::NotFound);
    }

    let branch_id = data
        .document_fork((channel_id, parent_id), user_id, json)
        .await?;

    let snapshot = srv.documents.get_snapshot((channel_id, parent_id)).await?;

    // use seq 0 for the initial snapshot of the new branch
    let snapshot_id = Uuid::now_v7();
    data.document_compact((channel_id, branch_id), snapshot_id, 0, snapshot)
        .await?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;

    s.broadcast(MessageSync::DocumentBranchCreate {
        branch: branch.clone(),
    })?;

    Ok(Json(branch))
}

/// Document branch merge
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
    Path((channel_id, branch_id)): Path<(ChannelId, DocumentBranchId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _json: Json<DocumentBranchMerge>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();
    let user_id = auth.user.id;

    let perms = srv.perms.for_channel(user_id, channel_id).await?;
    perms.ensure(Permission::DocumentEdit)?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;

    if branch.private && branch.creator_id != user_id {
        return Err(Error::NotFound);
    }

    if branch.creator_id != user_id {
        perms.ensure(Permission::ThreadManage)?;
    }

    if branch.default {
        return Err(Error::BadRequest("Cannot merge default branch".to_string()));
    }

    let parent_id = branch
        .parent_id
        .ok_or_else(|| Error::BadRequest("Branch has no parent".to_string()))?;
    let target_branch_id = parent_id.branch_id;

    let target_branch = data
        .document_branch_get(channel_id, target_branch_id)
        .await?;
    if target_branch.private && target_branch.creator_id != user_id {
        return Err(Error::NotFound);
    }

    let target_context = (channel_id, target_branch_id);
    let source_context = (channel_id, branch_id);

    let target_sv = srv.documents.get_state_vector(target_context).await?;
    let update = srv
        .documents
        .diff(source_context, Some(user_id), &target_sv)
        .await?;

    if !update.is_empty() {
        srv.documents
            .apply_update(target_context, user_id, None, &update)
            .await?;
    }

    data.document_branch_set_state(channel_id, branch_id, DocumentBranchState::Merged)
        .await?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;

    s.broadcast(MessageSync::DocumentBranchUpdate {
        branch: branch.clone(),
    })?;

    Ok(Json(branch))
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
        DocumentRevisionId::Revision { version_id } => (version_id.branch_id, version_id.seq),
        DocumentRevisionId::Tag { .. } => {
            return Err(Error::BadRequest("Cannot tag another tag".to_string()));
        }
    };

    let branch = data.document_branch_get(channel_id, branch_id).await?;
    if branch.private && branch.creator_id != user_id {
        return Err(Error::NotFound);
    }

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
    let user_id = auth.user.id;
    let data = s.data();
    let srv = s.services();

    let perms = srv.perms.for_channel(user_id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let tags = data
        .document_tag_list_by_document(channel_id, user_id)
        .await?;
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
    let user_id = auth.user.id;
    let srv = s.services();

    let perms = srv.perms.for_channel(user_id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let data = s.data();
    let tag = data.document_tag_get(tag_id).await?;

    let branch = data.document_branch_get(channel_id, tag.branch_id).await?;
    if branch.private && branch.creator_id != user_id {
        return Err(Error::NotFound);
    }

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

    let branch = data.document_branch_get(channel_id, tag.branch_id).await?;
    if branch.private && branch.creator_id != user_id {
        return Err(Error::NotFound);
    }

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

    let branch = data.document_branch_get(channel_id, tag.branch_id).await?;
    if branch.private && branch.creator_id != user_id {
        return Err(Error::NotFound);
    }

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

/// Document history
///
/// query edit history for a document
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
    Path((channel_id, branch_id)): Path<(ChannelId, DocumentBranchId)>,
    Query(query): Query<HistoryParams>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;
    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::NotFound);
    }

    let summary = srv
        .documents
        .query_history((channel_id, branch_id), query)
        .await?;

    let user_ids = summary.user_ids();
    let users = srv.users.get_many(&user_ids).await?;

    let channel = srv.channels.get(channel_id, Some(auth.user.id)).await?;
    let room_members = if let Some(room_id) = channel.room_id {
        data.room_member_get_many(room_id, &user_ids).await?
    } else {
        vec![]
    };

    let thread_members = data.thread_member_get_many(channel_id, &user_ids).await?;

    Ok(Json(HistoryPagination {
        changesets: summary.changesets,
        users,
        room_members,
        thread_members,
        document_tags: summary.tags,
    }))
}

/// Document crdt diff
///
/// get a crdt (yjs/yrs) snapshot since a state vector
#[utoipa::path(
    get,
    path = "/document/{channel_id}/branch/{branch_id}/crdt",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
        DocumentCrdtDiffParams,
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok", body = Vec<u8>),
    )
)]
async fn document_crdt_diff(
    Path((channel_id, branch_id)): Path<(ChannelId, DocumentBranchId)>,
    Query(query): Query<DocumentCrdtDiffParams>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;
    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::NotFound);
    }

    let sv = query.sv.unwrap_or(DocumentStateVector(vec![])).0;
    let update = srv
        .documents
        .diff((channel_id, branch_id), Some(auth.user.id), &sv)
        .await?;

    Ok(update.into_response())
}

/// Document crdt apply
///
/// apply a crdt (yjs/yrs) update
#[utoipa::path(
    patch,
    path = "/document/{channel_id}/branch/{branch_id}/crdt",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
    ),
    request_body(content = Vec<u8>, content_type = "application/octet-stream"),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_crdt_apply(
    Path((channel_id, branch_id)): Path<(ChannelId, DocumentBranchId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    body: Bytes,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::DocumentEdit)?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;
    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::NotFound);
    }

    let update_data = body.to_vec();
    srv.documents
        .apply_update((channel_id, branch_id), auth.user.id, None, &update_data)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Document content get
///
/// get document content for a specific revision as a serialized json document
#[utoipa::path(
    get,
    path = "/document/{channel_id}/revision/{revision_id}/content",
    params(
        ("channel_id", description = "Channel id"),
        ("revision_id", description = "Revision id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok", body = Serdoc),
    )
)]
async fn document_content_get(
    Path((channel_id, revision_id)): Path<(ChannelId, DocumentRevisionId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::ViewChannel)?;

    let branch_id = match revision_id {
        DocumentRevisionId::Branch { branch_id } => branch_id,
        _ => return Err(Error::Unimplemented),
    };

    let branch = data.document_branch_get(channel_id, branch_id).await?;
    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::NotFound);
    }

    let serdoc = srv.documents.get_content((channel_id, branch_id)).await?;
    Ok(Json(serdoc))
}

/// Document content put
///
/// replace the content of a document with a serialized json document. creates a new revision without overwriting existing history.
#[utoipa::path(
    put,
    path = "/document/{channel_id}/branch/{branch_id}/content",
    params(
        ("channel_id", description = "Channel id"),
        ("branch_id", description = "Branch id"),
    ),
    tags = ["document"],
    responses(
        (status = OK, description = "ok"),
    )
)]
async fn document_content_put(
    Path((channel_id, branch_id)): Path<(ChannelId, DocumentBranchId)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<SerdocPut>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_channel(auth.user.id, channel_id).await?;
    perms.ensure(Permission::DocumentEdit)?;

    let branch = data.document_branch_get(channel_id, branch_id).await?;
    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::NotFound);
    }

    srv.documents
        .set_content(
            (channel_id, branch_id),
            auth.user.id,
            Serdoc { root: json.root },
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(wiki_history))
        .routes(routes!(document_branch_list))
        .routes(routes!(document_branch_get))
        .routes(routes!(document_branch_update))
        .routes(routes!(document_branch_close))
        .routes(routes!(document_branch_fork))
        .routes(routes!(document_branch_merge))
        .routes(routes!(document_tag_create))
        .routes(routes!(document_tag_list))
        .routes(routes!(document_tag_get))
        .routes(routes!(document_tag_update))
        .routes(routes!(document_tag_delete))
        .routes(routes!(document_history))
        .routes(routes!(document_crdt_diff))
        .routes(routes!(document_crdt_apply))
        .routes(routes!(document_content_get))
        .routes(routes!(document_content_put))
}
