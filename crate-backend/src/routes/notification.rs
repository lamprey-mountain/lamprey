use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::notifications::{Notification, NotificationPagination, NotificationType};
use common::v1::types::util::Time;
use common::v1::types::{NotificationId, Permission};
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;

use crate::error::Result;
use crate::routes::util::Auth;
use crate::{routes2, ServerState};

/// Inbox get
///
/// List notifications
#[handler(routes::inbox_get)]
async fn inbox_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::inbox_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let notifications = s
        .data()
        .notification_list(auth.user.id, req.pagination, req.params)
        .await?;

    let mut channel_ids = std::collections::HashSet::new();
    for notif in &notifications.items {
        if let Some(channel_id) = notif.channel_id() {
            channel_ids.insert(channel_id);
        }
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
        if let (Some(channel_id), Some(message_id)) = (notif.channel_id(), notif.ty.message_id()) {
            if let Ok(mut message) = s.data().message_get(channel_id, message_id).await {
                s.presign_message(&mut message).await?;
                messages.push(message);
            }
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
#[handler(routes::inbox_post)]
async fn inbox_post(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::inbox_post::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let perms = s
        .services()
        .perms
        .for_channel(auth.user.id, req.notification.channel_id)
        .await?;
    perms.ensure(Permission::ChannelView)?;

    let room_id = s
        .services()
        .channels
        .get(req.notification.channel_id, Some(auth.user.id))
        .await
        .ok()
        .and_then(|ch| ch.room_id);

    let notif = Notification {
        id: NotificationId::new(),
        ty: NotificationType::Message {
            room_id,
            channel_id: req.notification.channel_id,
            message_id: req.notification.message_id,
        },
        added_at: req.notification.added_at.unwrap_or_else(Time::now_utc),
        read_at: None,
        note: None,
    };

    s.data()
        .notification_add(auth.user.id, notif.clone())
        .await?;

    Ok((StatusCode::CREATED, Json(notif)))
}

/// Inbox mark read
#[handler(routes::inbox_mark_read)]
async fn inbox_mark_read(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::inbox_mark_read::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data()
        .notification_mark_read(auth.user.id, req.mark_read)
        .await?;
    Ok(StatusCode::OK)
}

/// Inbox mark unread
#[handler(routes::inbox_mark_unread)]
async fn inbox_mark_unread(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::inbox_mark_unread::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data()
        .notification_mark_unread(auth.user.id, req.mark_unread)
        .await?;
    Ok(StatusCode::OK)
}

/// Inbox flush
///
/// Deletes read notifications from the inbox
#[handler(routes::inbox_flush)]
async fn inbox_flush(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::inbox_flush::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    s.data().notification_flush(auth.user.id, req.flush).await?;
    Ok(StatusCode::OK)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(inbox_get))
        .routes(routes2!(inbox_post))
        .routes(routes2!(inbox_mark_read))
        .routes(routes2!(inbox_mark_unread))
        .routes(routes2!(inbox_flush))
}
