use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::TypedHeader;
use common::v1::types::{AuditLog, AuditLogId};
use headers::ETag;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Result,
    types::{
        MessageSync, PaginationQuery, PaginationResponse, Permission, Room, RoomCreate, RoomId,
        RoomPatch,
    },
    Error, ServerState,
};

use super::util::{Auth, HeaderReason};

/// Create a room
#[utoipa::path(
    post,
    path = "/room",
    tags = ["room"],
)]
#[axum::debug_handler]
async fn room_create(
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RoomCreate>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let room = s.services().rooms.create(json, user_id).await?;
    s.broadcast(MessageSync::UpsertRoom { room: room.clone() })?;
    Ok((StatusCode::CREATED, Json(room)))
}

/// Get a room by its id
#[utoipa::path(
    get,
    path = "/room/{room_id}",
    tags = ["room"],
    params(("room_id", description = "Room id")),
    responses(
        (status = OK, description = "Get room success", body = Room),
        (status = NOT_MODIFIED, description = "Not modified"),
    )
)]
async fn room_get(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(user_id): Auth,
    headers: HeaderMap,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    let room = srv.rooms.get(room_id, Some(user_id)).await?;

    // TODO: use typedheader once the empty if-none-match bug is fixed
    // TODO: last-modified
    let etag = format!(r#"W/"{}""#, room.version_id);

    if let Some(if_none_match) = headers.get("if-none-match") {
        if if_none_match == &etag {
            return Ok(StatusCode::NOT_MODIFIED.into_response());
        }
    }

    let etag: ETag = etag.parse().unwrap();
    Ok((TypedHeader(etag), Json(room)).into_response())
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema, IntoParams, Validate)]
struct RoomListParams {
    /// what rooms to include. defaults to Default
    #[serde(default = "default_room_list_includes")]
    include: Vec<RoomListInclude>,
}

fn default_room_list_includes() -> Vec<RoomListInclude> {
    vec![RoomListInclude::Default]
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
enum RoomListInclude {
    /// include default rooms
    Default,

    /// include dm rooms
    Dm,

    /// include rooms you have were kicked or banned from, or left with ?soft=true
    Removed,

    /// include rooms that were archived
    Archived,
}

/// List visible rooms
#[utoipa::path(
    get,
    path = "/room",
    tags = ["room"],
    params(PaginationQuery<RoomId>),
    responses(
        (status = 200, description = "Paginate room success", body = PaginationResponse<Room>),
    )
)]
async fn room_list(
    Query(q): Query<PaginationQuery<RoomId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let res = data.room_list(user_id, q).await?;
    Ok(Json(res))
}

/// Edit a room
#[utoipa::path(
    patch,
    path = "/room/{room_id}",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["room"],
    responses(
        (status = OK, description = "edit success"),
        (status = NOT_MODIFIED, description = "no change"),
    )
)]
async fn room_edit(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<RoomPatch>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoomManage)?;
    let room = s.services().rooms.update(room_id, user_id, json).await?;
    let msg = MessageSync::UpsertRoom { room: room.clone() };
    s.broadcast_room(room_id, user_id, reason, msg).await?;
    Ok(Json(room))
}

/// Fetch audit logs
#[utoipa::path(
    get,
    path = "/room/{room_id}/logs",
    params(
        PaginationQuery<AuditLogId>,
        ("room_id", description = "Room id"),
    ),
    tags = ["room"],
    responses(
        (status = 200, description = "fetch audit logs success", body = PaginationResponse<AuditLog>),
    )
)]
#[deprecated = "use /audit-logs route"]
async fn room_audit_logs_old(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<AuditLogId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoomManage)?;
    let logs = data.audit_logs_room_fetch(room_id, paginate).await?;
    Ok(Json(logs))
}

/// Fetch audit logs
#[utoipa::path(
    get,
    path = "/room/{room_id}/audit-logs",
    params(
        PaginationQuery<AuditLogId>,
        ("room_id", description = "Room id"),
    ),
    tags = ["room"],
    responses(
        (status = 200, description = "fetch audit logs success", body = PaginationResponse<AuditLog>),
    )
)]
async fn room_audit_logs(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<AuditLogId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoomManage)?;
    let logs = data.audit_logs_room_fetch(room_id, paginate).await?;
    Ok(Json(logs))
}

/// Ack room (TODO)
///
/// Mark all threads in a room as read.
#[utoipa::path(
    put,
    path = "/room/{room_id}/ack",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["room"],
    responses(
        (status = OK, description = "success"),
    )
)]
async fn room_ack(
    Path(_room_id): Path<RoomId>,
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(room_create))
        .routes(routes!(room_get))
        .routes(routes!(room_list))
        .routes(routes!(room_edit))
        .routes(routes!(room_audit_logs))
        .routes(routes!(room_audit_logs_old))
        .routes(routes!(room_ack))
}
