use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::{
    MessageSync, MessageVerId, PaginationQuery, PaginationResponse, Thread, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// Dm initialize
///
/// Get or create a direct message thread.
#[utoipa::path(
    post,
    path = "/user/@self/dm/{target_id}",
    params(("target_id", description = "Target user's id")),
    tags = ["dm"],
    responses(
        (status = CREATED, description = "new dm created"),
        (status = OK, description = "already exists"),
    )
)]
async fn dm_init(
    Path(target_user_id): Path<UserId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let srv = s.services();
    let (thread, is_new) = srv.users.init_dm(auth_user.id, target_user_id).await?;
    s.broadcast(MessageSync::ThreadCreate {
        thread: thread.clone(),
    })?;
    if is_new {
        Ok((StatusCode::CREATED, Json(thread)))
    } else {
        Ok((StatusCode::OK, Json(thread)))
    }
}

/// Dm get
///
/// Get a direct message room.
#[utoipa::path(
    get,
    path = "/user/@self/dm/{target_id}",
    params(
        ("target_id", description = "Target user's id"),
    ),
    tags = ["dm"],
    responses(
        (status = OK, description = "success"),
    )
)]
async fn dm_get(
    Path(target_user_id): Path<UserId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let Some(thread_id) = data.dm_get(auth_user.id, target_user_id).await? else {
        return Err(Error::NotFound);
    };
    let srv = s.services();
    let thread = srv.threads.get(thread_id, Some(auth_user.id)).await?;
    Ok(Json(thread))
}

/// Dm list
///
/// List direct message threads. Ordered by the last message version id, so
/// recently active dms come first.
#[utoipa::path(
    get,
    path = "/user/{user_id}/dm",
    params(
        PaginationQuery<MessageVerId>,
        ("user_id", description = "user id"),
    ),
    tags = ["dm"],
    responses(
        (status = OK, body = PaginationResponse<Thread>, description = "success"),
    )
)]
async fn dm_list(
    Path(target_user_id): Path<UserIdReq>,
    Auth(auth_user): Auth,
    Query(q): Query<PaginationQuery<MessageVerId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth_user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    let data = s.data();
    let mut res = data.dm_list(auth_user.id, q).await?;

    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        threads.push(srv.threads.get(t.id, Some(auth_user.id)).await?);
    }
    res.items = threads;

    Ok(Json(res))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(dm_init))
        .routes(routes!(dm_get))
        .routes(routes!(dm_list))
}
