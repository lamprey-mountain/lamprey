use std::sync::Arc;

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::notifications::{
    InboxListParams, InboxThreadsParams, Notification, NotificationCreate, NotificationFlush,
    NotificationMarkRead,
};
use common::v1::types::{NotificationId, PaginationQuery, PaginationResponse, Thread, ThreadId};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::Result;
use crate::{Error, ServerState};

/// Inbox get (TODO)
///
/// List notifications
#[utoipa::path(
    get,
    path = "/inbox",
    params(PaginationQuery<MessageId>, InboxListParams),
    tags = ["inbox"],
    responses((status = OK, body = PaginationResponse<Notification>, description = "success"))
)]
async fn inbox_get(
    Auth(_auth_user_id): Auth,
    Query(_pagination): Query<PaginationQuery<NotificationId>>,
    Query(_params): Query<InboxListParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Inbox post (TODO)
///
/// Create a reminder for later
#[utoipa::path(
    post,
    path = "/inbox",
    tags = ["inbox"],
    responses((status = OK, body = Notification, description = "success"))
)]
async fn inbox_post(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<NotificationCreate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Inbox threads (TODO)
///
/// Get a list of all unread threads
// should i return messages in each thread? a PaginationResponse of PaginationResponses?
// maybe, it would save a round trip
// but what which messages do i return? last messages? new messages since unread marker? some context around the unread marker?
// this should probably be configurable via query parameter
// probably won't return messages and will let the client decide how to fetch messages
#[utoipa::path(
    get,
    path = "/inbox/threads",
    tags = ["inbox"],
    params(PaginationQuery<ThreadId>, InboxListParams, InboxThreadsParams),
    responses((status = OK, body = PaginationResponse<Thread>, description = "success"))
)]
async fn inbox_threads(
    Auth(_auth_user_id): Auth,
    Query(_pagination): Query<PaginationQuery<ThreadId>>,
    Query(_inbox_params): Query<InboxListParams>,
    Query(_thread_params): Query<InboxThreadsParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Inbox mark read (TODO)
#[utoipa::path(
    post,
    path = "/inbox/mark-read",
    tags = ["inbox"],
    responses((status = OK, body = (), description = "success"))
)]
async fn inbox_mark_read(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<NotificationMarkRead>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Inbox mark unread (TODO)
#[utoipa::path(
    post,
    path = "/inbox/mark-unread",
    tags = ["inbox"],
    responses((status = OK, body = (), description = "success"))
)]
async fn inbox_mark_unread(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<NotificationMarkRead>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Inbox flush (TODO)
///
/// Deletes read notifications from the inbox
#[utoipa::path(
    post,
    path = "/inbox/flush",
    tags = ["inbox"],
    responses((status = OK, body = (), description = "success"))
)]
async fn inbox_flush(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<NotificationFlush>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(inbox_get))
        .routes(routes!(inbox_post))
        .routes(routes!(inbox_threads))
        .routes(routes!(inbox_mark_read))
        .routes(routes!(inbox_mark_unread))
        .routes(routes!(inbox_flush))
}
