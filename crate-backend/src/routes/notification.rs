use std::sync::Arc;

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::notifications::{
    InboxListParams, InboxThreadsParams, Notification, NotificationCreate, NotificationFlush,
    NotificationMarkRead, NotificationReason,
};
use common::v1::types::{
    util::Time, NotificationId, PaginationQuery, PaginationResponse, Thread, ThreadId,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::Result;
use crate::ServerState;

/// Inbox get
///
/// List notifications
#[utoipa::path(
    get,
    path = "/inbox",
    params(PaginationQuery<NotificationId>, InboxListParams),
    tags = ["inbox"],
    responses((status = OK, body = PaginationResponse<Notification>, description = "success"))
)]
async fn inbox_get(
    Auth(auth_user): Auth,
    Query(pagination): Query<PaginationQuery<NotificationId>>,
    Query(params): Query<InboxListParams>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let res = s
        .data()
        .notification_list(auth_user.id, pagination, params)
        .await?;
    Ok(Json(res))
}

/// Inbox post
///
/// Create a reminder for later
#[utoipa::path(
    post,
    path = "/inbox",
    tags = ["inbox"],
    responses((status = CREATED, body = Notification, description = "success"))
)]
async fn inbox_post(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<NotificationCreate>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_thread(auth_user.id, json.thread_id)
        .await?;
    perms.ensure_view()?;

    let notif = Notification {
        id: NotificationId::new(),
        thread_id: json.thread_id,
        message_id: json.message_id,
        reason: NotificationReason::Reminder,
        added_at: json.added_at.unwrap_or_else(Time::now_utc),
    };

    s.data()
        .notification_add(auth_user.id, notif.clone())
        .await?;

    Ok((StatusCode::CREATED, Json(notif)))
}

/// Inbox threads
///
/// Get a list of all unread threads
#[utoipa::path(
    get,
    path = "/inbox/threads",
    tags = ["inbox"],
    params(PaginationQuery<ThreadId>, InboxListParams, InboxThreadsParams),
    responses((status = OK, body = PaginationResponse<Thread>, description = "success"))
)]
async fn inbox_threads(
    Auth(auth_user): Auth,
    Query(pagination): Query<PaginationQuery<ThreadId>>,
    Query(inbox_params): Query<InboxListParams>,
    Query(thread_params): Query<InboxThreadsParams>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let mut res = s
        .data()
        .notification_list_threads(auth_user.id, pagination, thread_params, inbox_params)
        .await?;

    for thread in &mut res.items {
        *thread = s
            .services()
            .threads
            .get(thread.id, Some(auth_user.id))
            .await?;
    }

    Ok(Json(res))
}

/// Inbox mark read
#[utoipa::path(
    post,
    path = "/inbox/mark-read",
    tags = ["inbox"],
    responses((status = OK, body = (), description = "success"))
)]
async fn inbox_mark_read(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<NotificationMarkRead>,
) -> Result<impl IntoResponse> {
    s.data().notification_mark_read(auth_user.id, json).await?;
    Ok(StatusCode::OK)
}

/// Inbox mark unread
#[utoipa::path(
    post,
    path = "/inbox/mark-unread",
    tags = ["inbox"],
    responses((status = OK, body = (), description = "success"))
)]
async fn inbox_mark_unread(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<NotificationMarkRead>,
) -> Result<impl IntoResponse> {
    s.data()
        .notification_mark_unread(auth_user.id, json)
        .await?;
    Ok(StatusCode::OK)
}

/// Inbox flush
///
/// Deletes read notifications from the inbox
#[utoipa::path(
    post,
    path = "/inbox/flush",
    tags = ["inbox"],
    responses((status = OK, body = (), description = "success"))
)]
async fn inbox_flush(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<NotificationFlush>,
) -> Result<impl IntoResponse> {
    s.data().notification_flush(auth_user.id, json).await?;
    Ok(StatusCode::OK)
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
