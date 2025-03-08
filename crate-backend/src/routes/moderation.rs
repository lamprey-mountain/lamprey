use std::sync::Arc;

use axum::{extract::State, Json};
use common::v1::types::moderation::{Report, ReportCreate};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::{Error, Result};
use crate::ServerState;

/// Report room (TODO)
///
/// Report a room
#[utoipa::path(
    post,
    path = "/room/{room_id}/report",
    params(("room_id", description = "Room id")),
    tags = ["moderation"],
    responses((status = OK, body = Report, description = "success"))
)]
async fn report_room(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<ReportCreate>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Report user (TODO)
///
/// Report a user
#[utoipa::path(
    post,
    path = "/user/{user_id}/report",
    params(("user_id", description = "user id")),
    tags = ["moderation"],
    responses((status = OK, body = Report, description = "success"))
)]
async fn report_user(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<ReportCreate>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Report media (TODO)
///
/// Report media
#[utoipa::path(
    post,
    path = "/media/{media_id}/report",
    params(("media_id", description = "media id")),
    tags = ["moderation"],
    responses((status = OK, body = Report, description = "success"))
)]
async fn report_media(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<ReportCreate>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Report thread (TODO)
///
/// Report a thread
#[utoipa::path(
    post,
    path = "/room/{room_id}/thread/{thread_id}/report",
    params(
        ("room_id", description = "room id"),
        ("thread_id", description = "thread id"),
    ),
    tags = ["moderation"],
    responses((status = OK, body = Report, description = "success"))
)]
async fn report_thread(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<ReportCreate>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Report message (TODO)
///
/// Report a message
#[utoipa::path(
    post,
    path = "/room/{room_id}/thread/{thread_id}/message/{message_id}/report",
    params(
        ("room_id", description = "room id"),
        ("thread_id", description = "thread id"),
        ("message_id", description = "message id"),
    ),
    tags = ["moderation"],
    responses((status = OK, body = Report, description = "success"))
)]
async fn report_message(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<ReportCreate>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(report_room))
        .routes(routes!(report_user))
        .routes(routes!(report_media))
        .routes(routes!(report_thread))
        .routes(routes!(report_message))
}
