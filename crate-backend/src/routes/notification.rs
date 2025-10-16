use std::sync::Arc;

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::notifications::{
    InboxListParams, InboxThreadsParams, Notification, NotificationCreate, NotificationFlush,
    NotificationMarkRead, NotificationPagination, NotificationReason,
};
use common::v1::types::PaginationResponse;
use common::v1::types::{
    util::Time, NotificationId, PaginationQuery, Permission, Thread, ThreadId,
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
    responses((status = OK, body = NotificationPagination, description = "success"))
)]
async fn inbox_get(
    Auth(auth_user): Auth,
    Query(pagination): Query<PaginationQuery<NotificationId>>,
    Query(params): Query<InboxListParams>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let notifications = s
        .data()
        .notification_list(auth_user.id, pagination, params)
        .await?;

    let mut thread_ids = std::collections::HashSet::new();
    for notif in &notifications.items {
        thread_ids.insert(notif.thread_id);
    }

    let srv = s.services();

    let mut threads = Vec::new();
    for thread_id in thread_ids {
        if let Ok(thread) = srv.threads.get(thread_id, Some(auth_user.id)).await {
            threads.push(thread);
        }
    }

    let mut room_ids = std::collections::HashSet::new();
    for thread in &threads {
        if let Some(room_id) = thread.room_id {
            room_ids.insert(room_id);
        }
    }

    let mut rooms = Vec::new();
    for room_id in room_ids {
        if let Ok(room) = srv.rooms.get(room_id, Some(auth_user.id)).await {
            rooms.push(room);
        }
    }

    let mut messages = Vec::new();
    for notif in &notifications.items {
        if let Ok(mut message) = s
            .data()
            .message_get(notif.thread_id, notif.message_id, auth_user.id)
            .await
        {
            s.presign_message(&mut message).await?;
            messages.push(message);
        }
    }

    let res = NotificationPagination {
        inner: notifications,
        threads,
        messages,
        rooms,
    };

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
    perms.ensure(Permission::ViewThread)?;

    let notif = Notification {
        id: NotificationId::new(),
        thread_id: json.thread_id,
        message_id: json.message_id,
        reason: NotificationReason::Reminder,
        added_at: json.added_at.unwrap_or_else(Time::now_utc),
        read_at: None,
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
