use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::{extract::State, Json};
use common::v1::types::notifications::{
    InboxFilters, InboxPatch, Notification, NotifsRoom, NotifsThread,
};
use common::v1::types::{PaginationQuery, PaginationResponse, RoomId, ThreadId, UserId};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::{Error, Result};
use crate::ServerState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, IntoParams)]
struct InboxListParams {
    include: InboxFilters,

    /// only include notifications from this room
    #[serde(default)]
    room_id: Vec<RoomId>,

    /// only include notifications from this thread
    #[serde(default)]
    thread_id: Vec<ThreadId>,
}

/// Inbox list (TODO)
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
async fn inbox_query(
    Auth(_auth_user_id): Auth,
    Query(_pagination): Query<PaginationQuery<UserId>>,
    Query(_params): Query<InboxListParams>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Inbox edit (TODO)
///
/// Edit notifications in the inbox.
#[utoipa::path(
    patch,
    path = "/inbox",
    tags = ["notification"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn inbox_patch(
    Auth(_auth_user_id): Auth,
    Query(_q): Query<PaginationQuery<UserId>>,
    State(_s): State<Arc<ServerState>>,
    Json(_body): Json<InboxPatch>,
) -> Result<Json<()>> {
    // how to handle partial failures?
    Err(Error::Unimplemented)
}

/// Notification room configure (TODO)
///
/// Edit notification settings for a room.
#[utoipa::path(
    patch,
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
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Notification thread configure (TODO)
///
/// Edit notification settings for a thread.
#[utoipa::path(
    patch,
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
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(inbox_query))
        .routes(routes!(inbox_patch))
        .routes(routes!(notification_room_configure))
        .routes(routes!(notification_thread_configure))
}
