use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::{extract::State, Json};
use common::v1::types::notifications::{Notification, NotifsRoom, NotifsThread};
use common::v1::types::{NotificationId, PaginationQuery, PaginationResponse, RoomId, ThreadId};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::Result;
use crate::{Error, ServerState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, IntoParams)]
struct InboxListParams {
    // TODO
    // /// only include notifications from this room
    // #[serde(default)]
    // room_id: Vec<RoomId>,

    // TODO
    // /// only include notifications from this thread
    // #[serde(default)]
    // thread_id: Vec<ThreadId>,
}

/// Inbox get (TODO)
///
/// List notifications.
#[utoipa::path(
    get,
    path = "/inbox",
    params(
        PaginationQuery<MessageId>,
        InboxListParams,
    ),
    tags = ["notification"],
    responses((status = OK, body = PaginationResponse<Notification>, description = "success"))
)]
async fn inbox_get(
    Auth(_auth_user_id): Auth,
    Query(_pagination): Query<PaginationQuery<NotificationId>>,
    Query(_params): Query<InboxListParams>,
    State(_s): State<Arc<ServerState>>,
    // ) -> Result<impl IntoResponse> {
) -> Result<Json<PaginationResponse<Notification>>> {
    Err(Error::Unimplemented)
}

// POST /inbox -- create a reminder
// DELETE /inbox/{notif_id} -- close notification

/// Notification room configure (TODO)
///
/// Edit notification settings for a room.
#[utoipa::path(
    put,
    path = "/room/{room_id}/config/notifications",
    params(("room_id", description = "Room id")),
    tags = ["notification"],
    responses((status = OK, body = NotifsRoom, description = "success")),
)]
async fn notification_room_configure(
    Path(_room_id): Path<RoomId>,
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<NotifsRoom>,
) -> Result<Json<NotifsRoom>> {
    todo!()
}

/// Notification thread configure (TODO)
///
/// Edit notification settings for a thread.
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/config/notifications",
    params(("thread_id", description = "Thread id")),
    tags = ["notification"],
    responses((status = OK, body = NotifsThread, description = "success")),
)]
async fn notification_thread_configure(
    Path(_thread_id): Path<ThreadId>,
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<NotifsThread>,
) -> Result<Json<NotifsThread>> {
    todo!()
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(inbox_get))
        .routes(routes!(notification_room_configure))
        .routes(routes!(notification_thread_configure))
}
