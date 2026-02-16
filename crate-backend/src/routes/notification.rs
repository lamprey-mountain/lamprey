use std::sync::Arc;

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::notifications::{
    InboxListParams, Notification, NotificationCreate, NotificationFlush, NotificationMarkRead,
    NotificationPagination, NotificationReason,
};
use common::v1::types::{util::Time, NotificationId, PaginationQuery, Permission};
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
    auth: Auth,
    Query(pagination): Query<PaginationQuery<NotificationId>>,
    Query(params): Query<InboxListParams>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let notifications = s
        .data()
        .notification_list(auth.user.id, pagination, params)
        .await?;

    let mut channel_ids = std::collections::HashSet::new();
    for notif in &notifications.items {
        channel_ids.insert(notif.channel_id);
    }

    let srv = s.services();

    let mut channels = Vec::new();
    for thread_id in channel_ids {
        if let Ok(thread) = srv.channels.get(thread_id, Some(auth.user.id)).await {
            channels.push(thread);
        }
    }

    let mut room_ids = std::collections::HashSet::new();
    for thread in &channels {
        if let Some(room_id) = thread.room_id {
            room_ids.insert(room_id);
        }
    }

    let mut rooms = Vec::new();
    for room_id in room_ids {
        if let Ok(room) = srv.rooms.get(room_id, Some(auth.user.id)).await {
            rooms.push(room);
        }
    }

    let mut messages = Vec::new();
    for notif in &notifications.items {
        if let Ok(mut message) = s
            .data()
            .message_get(notif.channel_id, notif.message_id, auth.user.id)
            .await
        {
            s.presign_message(&mut message).await?;
            messages.push(message);
        }
    }

    let res = NotificationPagination {
        notifications: notifications.items,
        total: notifications.total,
        has_more: notifications.has_more,
        cursor: notifications.cursor,
        channels,
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<NotificationCreate>,
) -> Result<impl IntoResponse> {
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, json.channel_id)
        .await?;
    perms.ensure(Permission::ViewChannel)?;

    let notif = Notification {
        id: NotificationId::new(),
        channel_id: json.channel_id,
        message_id: json.message_id,
        reason: NotificationReason::Reminder,
        added_at: json.added_at.unwrap_or_else(Time::now_utc),
        read_at: None,
    };

    s.data()
        .notification_add(auth.user.id, notif.clone())
        .await?;

    Ok((StatusCode::CREATED, Json(notif)))
}

/// Inbox mark read
#[utoipa::path(
    post,
    path = "/inbox/mark-read",
    tags = ["inbox"],
    responses((status = OK, body = (), description = "success"))
)]
async fn inbox_mark_read(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<NotificationMarkRead>,
) -> Result<impl IntoResponse> {
    s.data().notification_mark_read(auth.user.id, json).await?;
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<NotificationMarkRead>,
) -> Result<impl IntoResponse> {
    s.data()
        .notification_mark_unread(auth.user.id, json)
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<NotificationFlush>,
) -> Result<impl IntoResponse> {
    s.data().notification_flush(auth.user.id, json).await?;
    Ok(StatusCode::OK)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(inbox_get))
        .routes(routes!(inbox_post))
        .routes(routes!(inbox_mark_read))
        .routes(routes!(inbox_mark_unread))
        .routes(routes!(inbox_flush))
}
