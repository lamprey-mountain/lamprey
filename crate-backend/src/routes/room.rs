use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::TypedHeader;
use common::v1::types::{AuditLogEntry, AuditLogEntryId, RoomMetrics, UserId};
use headers::ETag;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Result,
    routes::util::AuthSudo,
    types::{
        MediaLinkType, MessageSync, PaginationQuery, PaginationResponse, Permission, Room,
        RoomCreate, RoomId, RoomPatch,
    },
    Error, ServerState,
};

use super::util::{Auth, HeaderReason};

/// Room create
#[utoipa::path(
    post,
    path = "/room",
    tags = ["room"],
)]
async fn room_create(
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RoomCreate>,
) -> Result<impl IntoResponse> {
    json.validate()?;

    // FIXME: run this in a transaction
    let icon = json.icon;
    if let Some(media_id) = icon {
        let data = s.data();
        let (media, _) = data.media_select(media_id).await?;
        if !matches!(
            media.source.info,
            common::v1::types::MediaTrackInfo::Image(_)
        ) {
            return Err(Error::BadStatic("media not an image"));
        }
        if media.source.size > 1024 * 256 {
            return Err(Error::BadStatic(
                "media is too big (max file size is 256KiB)",
            ));
        }
        if !data.media_link_select(media_id).await?.is_empty() {
            return Err(Error::BadStatic("media already used"));
        }
    }

    let room = s.services().rooms.create(json, user_id).await?;
    if let Some(media_id) = icon {
        let data = s.data();
        data.media_link_insert(media_id, *room.id, MediaLinkType::AvatarRoom)
            .await?;
    }
    s.broadcast(MessageSync::RoomCreate { room: room.clone() })?;

    Ok((StatusCode::CREATED, Json(room)))
}

/// Room get
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

/// Room list
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

/// Room edit
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

    let icon = json.icon;
    if let Some(Some(media_id)) = icon {
        let data = s.data();
        let (media, _) = data.media_select(media_id).await?;
        if !matches!(
            media.source.info,
            common::v1::types::MediaTrackInfo::Image(_)
        ) {
            return Err(Error::BadStatic("media not an image"));
        }
        if media.source.size > 1024 * 256 {
            return Err(Error::BadStatic(
                "media is too big (max file size is 256KiB)",
            ));
        }
        if !data.media_link_select(media_id).await?.is_empty() {
            return Err(Error::BadStatic("media already used"));
        }
    }

    let room = s
        .services()
        .rooms
        .update(room_id, user_id, json, reason.clone())
        .await?;
    let msg = MessageSync::RoomUpdate { room: room.clone() };
    s.broadcast_room(room_id, user_id, msg).await?;
    Ok(Json(room))
}

/// Room audit logs
#[utoipa::path(
    get,
    path = "/room/{room_id}/audit-logs",
    params(
        PaginationQuery<AuditLogId>,
        ("room_id", description = "Room id"),
    ),
    tags = ["room"],
    responses(
        (status = 200, description = "fetch audit logs success", body = PaginationResponse<AuditLogEntry>),
    )
)]
async fn room_audit_logs(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<AuditLogEntryId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ViewAuditLog)?;
    let logs = data.audit_logs_room_fetch(room_id, paginate).await?;
    Ok(Json(logs))
}

/// Room ack (TODO)
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

/// Room metrics
///
/// Get metrics for a room
#[utoipa::path(
    get,
    path = "/room/{room_id}/metrics",
    params(("room_id", description = "Room id")),
    tags = ["room"],
    responses((status = OK, description = "success", body = RoomMetrics))
)]
async fn room_metrics(
    Path(room_id): Path<RoomId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ViewAuditLog)?;
    let metrics = data.room_metrics(room_id).await?;
    Ok(Json(metrics))
}

/// Room transfer ownership
#[utoipa::path(
    post,
    path = "/room/{room_id}/transfer-ownership",
    params(("room_id", description = "Room id")),
    tags = ["room"],
    responses((status = OK, description = "success"))
)]
async fn room_transfer_ownership(
    Path(room_id): Path<RoomId>,
    AuthSudo(auth_user_id): AuthSudo,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<TransferOwnership>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let data = s.data();
    let target_user_id = json.owner_id;

    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    let room_start = srv.rooms.get(room_id, Some(auth_user_id)).await?;
    if room_start.owner_id != Some(auth_user_id) {
        return Err(Error::BadStatic("you aren't the room owner"));
    }

    data.room_set_owner(room_id, target_user_id).await?;
    srv.perms.invalidate_room(auth_user_id, room_id).await;
    srv.perms.invalidate_room(target_user_id, room_id).await;
    srv.rooms.invalidate(room_id).await;
    let room = srv.rooms.get(room_id, Some(auth_user_id)).await?;
    let msg = MessageSync::RoomUpdate { room: room.clone() };
    s.broadcast_room(room_id, auth_user_id, msg).await?;
    Ok(Json(room))
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
struct TransferOwnership {
    owner_id: UserId,
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(room_create))
        .routes(routes!(room_get))
        .routes(routes!(room_list))
        .routes(routes!(room_edit))
        .routes(routes!(room_audit_logs))
        .routes(routes!(room_ack))
        .routes(routes!(room_metrics))
        .routes(routes!(room_transfer_ownership))
}
