use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::document::serialized::Serdoc;
use common::v1::types::document::{DocumentBranchState, DocumentRevisionId, HistoryPagination};
use common::v1::types::{MessageSync, Permission};
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Wiki history
#[handler(routes::wiki_history)]
async fn wiki_history(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::wiki_history::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();

    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::ChannelView)
        .check()?;

    let summary = srv
        .documents
        .query_wiki_history(req.channel_id, req.query)
        .await?;

    let user_ids = summary.user_ids();
    let users = srv.users.get_many(&user_ids).await?;

    let channel = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    let room_members = if let Some(room_id) = channel.room_id {
        data.room_member_get_many(room_id, &user_ids).await?
    } else {
        vec![]
    };

    let thread_members = data
        .thread_member_get_many(req.channel_id, &user_ids)
        .await?;

    Ok(Json(HistoryPagination {
        changesets: summary.changesets,
        users,
        room_members,
        thread_members,
        document_tags: summary.tags,
    }))
}

/// Document branch list
#[handler(routes::document_branch_list)]
async fn document_branch_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_branch_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();

    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::ChannelView)
        .check()?;

    let branches = data
        .document_branch_paginate(req.channel_id, auth.user.id, req.query, req.pagination)
        .await?;

    Ok(Json(branches))
}

/// Document branch get
#[handler(routes::document_branch_get)]
async fn document_branch_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_branch_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();

    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::ChannelView)
        .check()?;

    let branch = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;

    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    Ok(Json(branch))
}

/// Document branch update
#[handler(routes::document_branch_update)]
async fn document_branch_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_branch_update::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = &srv.perms;
    let mut perms = perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::DocumentEdit);

    let branch_before = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;

    if branch_before.creator_id != auth.user.id {
        if branch_before.private {
            return Err(Error::ApiError(
                common::v1::types::error::ApiError::from_code(
                    common::v1::types::error::ErrorCode::UnknownDocumentBranch,
                ),
            ));
        }
        perms.needs(Permission::ThreadManage);
    }
    perms.check()?;

    data.document_branch_update(req.channel_id, req.branch_id, req.patch)
        .await?;

    let branch = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;

    s.broadcast(MessageSync::DocumentBranchUpdate {
        branch: branch.clone(),
    })?;

    Ok(Json(branch))
}

/// Document branch close
#[handler(routes::document_branch_close)]
async fn document_branch_close(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_branch_close::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = &srv.perms;
    let mut perms = perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::DocumentEdit);

    let branch = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;

    if branch.default {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::CannotCloseDefaultBranch,
            ),
        ));
    }

    if branch.creator_id != auth.user.id {
        if branch.private {
            return Err(Error::ApiError(
                common::v1::types::error::ApiError::from_code(
                    common::v1::types::error::ErrorCode::UnknownDocumentBranch,
                ),
            ));
        }
        perms.needs(Permission::ThreadManage);
    }
    perms.check()?;

    data.document_branch_set_state(req.channel_id, req.branch_id, DocumentBranchState::Closed)
        .await?;

    let branch = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;

    s.broadcast(MessageSync::DocumentBranchDelete {
        channel_id: req.channel_id,
        branch_id: req.branch_id,
    })?;

    Ok(Json(branch))
}

/// Document branch fork
#[handler(routes::document_branch_fork)]
async fn document_branch_fork(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_branch_fork::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();
    let user_id = auth.user.id;

    let perms = &srv.perms;
    let mut perms = perms
        .for_channel3(Some(user_id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::ChannelView);
    perms.needs(Permission::DocumentEdit);

    let parent_branch = data
        .document_branch_get(req.channel_id, req.parent_id)
        .await?;
    if parent_branch.private && parent_branch.creator_id != user_id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    let branch_id = data
        .document_fork((req.channel_id, req.parent_id), user_id, req.branch)
        .await?;

    let snapshot = srv
        .documents
        .get_snapshot((req.channel_id, req.parent_id))
        .await?;

    // use seq 0 for the initial snapshot of the new branch
    let snapshot_id = Uuid::now_v7();
    data.document_compact((req.channel_id, branch_id), snapshot_id, 0, snapshot)
        .await?;

    let branch = data.document_branch_get(req.channel_id, branch_id).await?;

    s.broadcast(MessageSync::DocumentBranchCreate {
        branch: branch.clone(),
    })?;

    Ok(Json(branch))
}

/// Document branch merge
#[handler(routes::document_branch_merge)]
async fn document_branch_merge(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_branch_merge::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();
    let user_id = auth.user.id;

    let perms = &srv.perms;
    let mut perms = perms
        .for_channel3(Some(user_id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::DocumentEdit);

    let branch = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;

    if branch.private && branch.creator_id != user_id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    if branch.creator_id != user_id {
        perms.needs(Permission::ThreadManage);
    }

    if branch.default {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::CannotMergeDefaultBranch,
            ),
        ));
    }

    let parent_id = branch.parent_id.ok_or_else(|| {
        Error::ApiError(common::v1::types::error::ApiError::from_code(
            common::v1::types::error::ErrorCode::BranchHasNoParent,
        ))
    })?;
    let target_branch_id = parent_id.branch_id;

    let target_branch = data
        .document_branch_get(req.channel_id, target_branch_id)
        .await?;
    if target_branch.private && target_branch.creator_id != user_id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    let target_context = (req.channel_id, target_branch_id);
    let source_context = (req.channel_id, req.branch_id);

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

    data.document_branch_set_state(req.channel_id, req.branch_id, DocumentBranchState::Merged)
        .await?;

    let branch = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;

    s.broadcast(MessageSync::DocumentBranchUpdate {
        branch: branch.clone(),
    })?;

    Ok(Json(branch))
}

/// Document tag create
#[handler(routes::document_tag_create)]
async fn document_tag_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_tag_create::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let user_id = auth.user.id;
    let data = s.data();
    let srv = s.services();

    let perms = &srv.perms;
    let mut perms = perms
        .for_channel3(Some(user_id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::DocumentEdit);

    let (branch_id, revision_seq) = match req.tag.revision {
        common::v1::types::document::DocumentRevisionId::Branch { branch_id: _ } => {
            // TODO: implement tagging branch heads
            return Err(Error::Unimplemented);
        }
        common::v1::types::document::DocumentRevisionId::Revision { version_id } => {
            (version_id.branch_id, version_id.seq)
        }
        common::v1::types::document::DocumentRevisionId::Tag { .. } => {
            return Err(Error::ApiError(
                common::v1::types::error::ApiError::from_code(
                    common::v1::types::error::ErrorCode::CannotTagAnotherTag,
                ),
            ));
        }
    };

    let branch = data.document_branch_get(req.channel_id, branch_id).await?;
    if branch.private && branch.creator_id != user_id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    let tag_id = data
        .document_tag_create(
            branch_id,
            user_id,
            req.tag.summary,
            req.tag.description,
            revision_seq,
        )
        .await?;

    let tag = data.document_tag_get(tag_id).await?;

    s.broadcast(MessageSync::DocumentTagCreate {
        channel_id: req.channel_id,
        tag: tag.clone(),
    })?;

    Ok(Json(tag))
}

/// Document tag list
#[handler(routes::document_tag_list)]
async fn document_tag_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_tag_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let user_id = auth.user.id;
    let data = s.data();
    let srv = s.services();

    srv.perms
        .for_channel3(Some(user_id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let tags = data
        .document_tag_list_by_document(req.channel_id, user_id)
        .await?;
    Ok(Json(tags))
}

/// Document tag get
#[handler(routes::document_tag_get)]
async fn document_tag_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_tag_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let user_id = auth.user.id;
    let srv = s.services();

    srv.perms
        .for_channel3(Some(user_id), req.channel_id)
        .await?
        .ensure_view()?
        .check()?;

    let data = s.data();
    let tag = data.document_tag_get(req.tag_id).await?;

    let branch = data
        .document_branch_get(req.channel_id, tag.branch_id)
        .await?;
    if branch.private && branch.creator_id != user_id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    Ok(Json(tag))
}

/// Document tag update
#[handler(routes::document_tag_update)]
async fn document_tag_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_tag_update::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let user_id = auth.user.id;
    let srv = s.services();
    let data = s.data();

    let perms = &srv.perms;
    let mut perms = perms
        .for_channel3(Some(user_id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::DocumentEdit);

    let tag = data.document_tag_get(req.tag_id).await?;

    let branch = data
        .document_branch_get(req.channel_id, tag.branch_id)
        .await?;
    if branch.private && branch.creator_id != user_id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    if tag.creator_id != Some(user_id) {
        perms.needs(Permission::ThreadManage);
    }

    perms.check()?;

    data.document_tag_update(req.tag_id, req.tag.summary, req.tag.description)
        .await?;

    let updated_tag = data.document_tag_get(req.tag_id).await?;

    s.broadcast(MessageSync::DocumentTagUpdate {
        channel_id: req.channel_id,
        tag: updated_tag.clone(),
    })?;

    Ok(Json(updated_tag))
}

/// Document tag delete
#[handler(routes::document_tag_delete)]
async fn document_tag_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_tag_delete::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;

    let user_id = auth.user.id;
    let srv = s.services();
    let data = s.data();

    let perms = &srv.perms;
    let mut perms = perms
        .for_channel3(Some(user_id), req.channel_id)
        .await?
        .ensure_view()?;
    perms.needs(Permission::DocumentEdit);

    let tag = data.document_tag_get(req.tag_id).await?;

    let branch = data
        .document_branch_get(req.channel_id, tag.branch_id)
        .await?;
    if branch.private && branch.creator_id != user_id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    if tag.creator_id != Some(user_id) {
        perms.needs(Permission::ThreadManage);
    }

    perms.check()?;

    let branch_id = tag.branch_id;

    data.document_tag_delete(req.tag_id).await?;

    s.broadcast(MessageSync::DocumentTagDelete {
        channel_id: req.channel_id,
        branch_id,
        tag_id: req.tag_id,
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Document history
#[handler(routes::document_history)]
async fn document_history(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_history::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();

    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::ChannelView)
        .check()?;

    let branch = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;
    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    let summary = srv
        .documents
        .query_history((req.channel_id, req.branch_id), req.query)
        .await?;

    let user_ids = summary.user_ids();
    let users = srv.users.get_many(&user_ids).await?;

    let channel = srv.channels.get(req.channel_id, Some(auth.user.id)).await?;
    let room_members = if let Some(room_id) = channel.room_id {
        data.room_member_get_many(room_id, &user_ids).await?
    } else {
        vec![]
    };

    let thread_members = data
        .thread_member_get_many(req.channel_id, &user_ids)
        .await?;

    Ok(Json(HistoryPagination {
        changesets: summary.changesets,
        users,
        room_members,
        thread_members,
        document_tags: summary.tags,
    }))
}

/// Document CRDT diff
#[handler(routes::document_crdt_diff)]
async fn document_crdt_diff(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_crdt_diff::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::ChannelView)
        .check()?;

    let branch = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;
    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    let sv = req
        .params
        .sv
        .unwrap_or(common::v1::types::document::DocumentStateVector(vec![]))
        .0;
    let update = srv
        .documents
        .diff((req.channel_id, req.branch_id), Some(auth.user.id), &sv)
        .await?;

    Ok(update)
}

/// Document CRDT apply
#[handler(routes::document_crdt_apply)]
async fn document_crdt_apply(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_crdt_apply::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::DocumentEdit)
        .check()?;

    let branch = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;
    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    let update_data = req.data;
    srv.documents
        .apply_update(
            (req.channel_id, req.branch_id),
            auth.user.id,
            None,
            update_data.as_ref(),
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Document content get
#[handler(routes::document_content_get)]
async fn document_content_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_content_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    let data = s.data();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::ChannelView)
        .check()?;

    let (branch_id, seq) = match req.revision_id {
        DocumentRevisionId::Branch { branch_id } => (branch_id, None),
        DocumentRevisionId::Revision { version_id } => (version_id.branch_id, Some(version_id.seq)),
        DocumentRevisionId::Tag { tag_id } => {
            let tag = data.document_tag_get(tag_id).await?;
            (tag.branch_id, Some(tag.revision_seq as u64))
        }
    };

    let branch = data.document_branch_get(req.channel_id, branch_id).await?;
    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    let serdoc = match seq {
        Some(seq) => {
            srv.documents
                .get_content_at_seq((req.channel_id, branch_id), seq)
                .await?
        }
        None => {
            srv.documents
                .get_content((req.channel_id, branch_id))
                .await?
        }
    };

    Ok(Json(serdoc))
}

/// Document content put
#[handler(routes::document_content_put)]
async fn document_content_put(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::document_content_put::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let data = s.data();
    srv.perms
        .for_channel3(Some(auth.user.id), req.channel_id)
        .await?
        .ensure_view()?
        .needs(Permission::DocumentEdit)
        .check()?;

    let branch = data
        .document_branch_get(req.channel_id, req.branch_id)
        .await?;
    if branch.private && branch.creator_id != auth.user.id {
        return Err(Error::ApiError(
            common::v1::types::error::ApiError::from_code(
                common::v1::types::error::ErrorCode::UnknownDocumentBranch,
            ),
        ));
    }

    srv.documents
        .set_content(
            (req.channel_id, req.branch_id),
            auth.user.id,
            Serdoc {
                root: req.content.root,
            },
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(wiki_history))
        .routes(routes2!(document_branch_list))
        .routes(routes2!(document_branch_get))
        .routes(routes2!(document_branch_update))
        .routes(routes2!(document_branch_close))
        .routes(routes2!(document_branch_fork))
        .routes(routes2!(document_branch_merge))
        .routes(routes2!(document_tag_create))
        .routes(routes2!(document_tag_list))
        .routes(routes2!(document_tag_get))
        .routes(routes2!(document_tag_update))
        .routes(routes2!(document_tag_delete))
        .routes(routes2!(document_history))
        .routes(routes2!(document_crdt_diff))
        .routes(routes2!(document_crdt_apply))
        .routes(routes2!(document_content_get))
        .routes(routes2!(document_content_put))
}
