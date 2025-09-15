use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::{
    MessageSync, PaginationQuery, PaginationResponse, Permission, ThreadId, ThreadMember,
    ThreadMemberPut, ThreadMembership, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::{Auth, HeaderReason};
use crate::error::{Error, Result};

/// Thread member list
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/member",
    params(
        PaginationQuery<UserId>,
        ("thread_id" = ThreadId, description = "Thread id"),
    ),
    tags = ["thread_member"],
    responses(
        (status = OK, body = PaginationResponse<ThreadMember>, description = "success"),
    )
)]
pub async fn thread_member_list(
    Path(thread_id): Path<ThreadId>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_thread(user_id, thread_id).await?;
    perms.ensure_view()?;
    let res = d.thread_member_list(thread_id, paginate).await?;
    Ok(Json(res))
}

/// Thread member get
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/member/{user_id}",
    params(
        ("thread_id" = ThreadId, description = "Thread id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["thread_member"],
    responses(
        (status = OK, body = ThreadMember, description = "success"),
    )
)]
pub async fn thread_member_get(
    Path((thread_id, target_user_id)): Path<(ThreadId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s
        .services()
        .perms
        .for_thread(auth_user_id, thread_id)
        .await?;
    perms.ensure_view()?;
    let res = d.thread_member_get(thread_id, target_user_id).await?;
    // TODO: return `Ban`s
    if !matches!(res.membership, ThreadMembership::Join { .. }) {
        Err(Error::NotFound)
    } else {
        Ok(Json(res))
    }
}

/// Thread member add
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/member/{user_id}",
    params(
        ("thread_id" = ThreadId, description = "Thread id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["thread_member"],
    responses(
        (status = OK, body = ThreadMember, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
pub async fn thread_member_add(
    Path((thread_id, target_user_id)): Path<(ThreadId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(_reason): HeaderReason,
    Json(json): Json<ThreadMemberPut>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    if target_user_id != auth_user_id {
        perms.ensure(Permission::MemberKick)?;
    }

    let thread = srv.threads.get(thread_id, Some(auth_user_id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }

    let start = d.thread_member_get(thread_id, target_user_id).await.ok();
    d.thread_member_put(thread_id, target_user_id, ThreadMemberPut {})
        .await?;
    let res = d.thread_member_get(thread_id, target_user_id).await?;
    if start.is_some_and(|s| s == res) {
        Ok(StatusCode::NOT_MODIFIED.into_response())
    } else {
        s.broadcast_thread(
            thread_id,
            auth_user_id,
            MessageSync::ThreadMemberUpsert {
                member: res.clone(),
            },
        )
        .await?;
        Ok(Json(res).into_response())
    }
}

/// Thread member delete (kick/leave)
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/member/{user_id}",
    params(
        ("thread_id" = ThreadId, description = "Thread id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["thread_member"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
pub async fn thread_member_delete(
    Path((thread_id, target_user_id)): Path<(ThreadId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    HeaderReason(_reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let srv = s.services();
    let perms = srv.perms.for_thread(auth_user_id, thread_id).await?;
    perms.ensure_view()?;
    if target_user_id != auth_user_id {
        perms.ensure(Permission::MemberKick)?;
    }

    let thread = srv.threads.get(thread_id, Some(auth_user_id)).await?;
    if thread.archived_at.is_some() {
        return Err(Error::BadStatic("thread is archived"));
    }
    if thread.deleted_at.is_some() {
        return Err(Error::BadStatic("thread is removed"));
    }
    if thread.locked {
        perms.ensure(Permission::ThreadLock)?;
    }

    let start = d.thread_member_get(thread_id, target_user_id).await?;
    if !matches!(start.membership, ThreadMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    d.thread_member_set_membership(thread_id, target_user_id, ThreadMembership::Leave {})
        .await?;
    s.services()
        .perms
        .invalidate_thread(target_user_id, thread_id);
    let res = d.thread_member_get(thread_id, target_user_id).await?;
    s.broadcast_thread(
        thread_id,
        auth_user_id,
        MessageSync::ThreadMemberUpsert { member: res },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(thread_member_list))
        .routes(routes!(thread_member_get))
        .routes(routes!(thread_member_add))
        .routes(routes!(thread_member_delete))
}
