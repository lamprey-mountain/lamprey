use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::{
    Channel, MessageSync, MessageVerId, PaginationQuery, PaginationResponse, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::Auth2;
use crate::error::{Error, Result};

// TODO: merge with channel_create_dm
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
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let target_user = s.data().user_get(target_user_id).await?;
    if !target_user.can_dm() {
        return Err(Error::BadStatic("cannot dm this user"));
    }
    let (thread, is_new) = srv.users.init_dm(auth.user.id, target_user_id).await?;
    s.broadcast(MessageSync::ChannelCreate {
        channel: Box::new(thread.clone()),
    })?;
    if is_new {
        Ok((StatusCode::CREATED, Json(thread)))
    } else {
        Ok((StatusCode::OK, Json(thread)))
    }
}

// TODO: move to channels.rs
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
    auth: Auth2,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let Some(thread_id) = data.dm_get(auth.user.id, target_user_id).await? else {
        return Err(Error::NotFound);
    };
    let srv = s.services();
    let thread = srv.channels.get(thread_id, Some(auth.user.id)).await?;
    Ok(Json(thread))
}

// TODO: move to channels.rs
// TODO: rename to /api/v1/user/@self/channel
/// Dm list
///
/// List direct message channels. Ordered by the last message version id, so
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
        (status = OK, body = PaginationResponse<Channel>, description = "success"),
    )
)]
async fn dm_list(
    Path(target_user_id): Path<UserIdReq>,
    auth: Auth2,
    Query(q): Query<PaginationQuery<MessageVerId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    let data = s.data();
    let mut res = data.dm_list(auth.user.id, q).await?;

    let srv = s.services();
    let mut threads = vec![];
    for t in &res.items {
        threads.push(srv.channels.get(t.id, Some(auth.user.id)).await?);
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
