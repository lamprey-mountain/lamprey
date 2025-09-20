use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::moderation::{Report, ReportCreate};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::{Error, Result};
use crate::ServerState;

/// Report create server (TODO)
///
/// Create and send a report to the server operators
#[utoipa::path(
    post,
    path = "/server/report",
    tags = ["moderation"],
    responses((status = OK, body = Report, description = "success"))
)]
async fn report_create_server(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<ReportCreate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// Report create room (TODO)
///
/// Create and send a report to the room admins/moderators
#[utoipa::path(
    post,
    path = "/room/{room_id}/report",
    params(("room_id", description = "Room id")),
    tags = ["moderation"],
    responses((status = OK, body = Report, description = "success"))
)]
async fn report_create_room(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<ReportCreate>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

// NOTE: do i need routes for listing reports? if reports are special types of threads, maybe not?
// i could have GET /server/report and GET /room/{room_id}/report paginate reports you sent
// paginating all reports for mods is kind of useless considering reports will be threads

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(report_create_server))
        .routes(routes!(report_create_room))
}
