use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::TypedHeader;
use common::v1::types::{
    application::Integration, ApplicationId, AuditLogEntry, AuditLogEntryId, AuditLogEntryType,
    RoomMetrics, RoomType, UserId, SERVER_ROOM_ID,
};
use headers::ETag;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Result,
    routes::util::AuthSudo,
    types::{
        DbRoomCreate, MediaLinkType, MessageSync, PaginationQuery, PaginationResponse, Permission,
        Room, RoomCreate, RoomId, RoomPatch,
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RoomCreate>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
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
    }

    let extra = DbRoomCreate {
        id: None,
        ty: RoomType::Default,
        welcome_thread_id: None,
    };
    let room = s.services().rooms.create(json, auth_user.id, extra).await?;
    if let Some(media_id) = icon {
        let data = s.data();
        data.media_link_create_exclusive(media_id, *room.id, MediaLinkType::AvatarRoom)
            .await?;
    }

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
    Auth(user): Auth,
    headers: HeaderMap,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_room(user.id, room_id).await?;
    perms.ensure_view()?;
    let room = srv.rooms.get(room_id, Some(user.id)).await?;

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

/// Room list
///
/// List rooms. If the user is an admin, lists all rooms on the server.
/// Otherwise, lists rooms the user is a member of.
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
    Auth(user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let is_admin = srv
        .perms
        .for_room(user.id, SERVER_ROOM_ID)
        .await?
        .has(Permission::Admin);

    if is_admin {
        let rooms = data.room_list_all(q).await?;
        Ok(Json(rooms))
    } else {
        Err(Error::MissingPermissions)
    }
}

/// Room edit
#[utoipa::path(
    patch,
    path = "/room/{room_id}",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["room", "badge.perm.RoomManage"],
    responses(
        (status = OK, description = "edit success"),
        (status = NOT_MODIFIED, description = "no change"),
    )
)]
async fn room_edit(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<RoomPatch>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    json.validate()?;
    let perms = s.services().perms.for_room(auth_user.id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoomManage)?;

    if let Some(Some(media_id)) = json.icon {
        let data = s.data();
        let (media, _) = data.media_select(media_id).await?;
        if !matches!(
            media.source.info,
            common::v1::types::MediaTrackInfo::Image(_)
        ) {
            return Err(Error::BadStatic("media not an image"));
        }
    }

    let room = s
        .services()
        .rooms
        .update(room_id, auth_user.id, json.clone(), reason.clone())
        .await?;

    if let Some(maybe_media_id) = json.icon {
        let data = s.data();
        data.media_link_delete(room_id.into_inner(), MediaLinkType::AvatarRoom)
            .await?;
        if let Some(media_id) = maybe_media_id {
            data.media_link_create_exclusive(
                media_id,
                room_id.into_inner(),
                MediaLinkType::AvatarRoom,
            )
            .await?;
        }
    }

    let msg = MessageSync::RoomUpdate { room: room.clone() };
    s.broadcast_room(room_id, auth_user.id, msg).await?;
    Ok(Json(room))
}

/// Room delete
#[utoipa::path(
    delete,
    path = "/room/{room_id}",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["room", "badge.sudo"],
    responses((status = OK, description = "success")),
)]
async fn room_delete(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;
    let is_admin = perms.has(Permission::Admin);

    let room = srv.rooms.get(room_id, None).await?;
    if room.owner_id != Some(auth_user.id) && !is_admin {
        return Err(Error::BadStatic("you aren't the room owner"));
    }

    s.broadcast_room(room_id, auth_user.id, MessageSync::RoomDelete { room_id })
        .await?;

    data.room_delete(room_id).await?;
    srv.rooms.invalidate(room_id).await;
    srv.perms.invalidate_room_all(room_id);

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomDelete { room_id },
    })
    .await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomDelete { room_id },
    })
    .await?;

    Ok(())
}

/// Room undelete
#[utoipa::path(
    post,
    path = "/room/{room_id}/undelete",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["room", "badge.admin_only", "badge.perm.Admin"],
    responses((status = OK, description = "success")),
)]
async fn room_undelete(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::Admin)?;

    data.room_undelete(room_id).await?;
    srv.rooms.invalidate(room_id).await;
    srv.perms.invalidate_room_all(room_id);

    let room = srv.rooms.get(room_id, None).await?;
    s.broadcast_room(room_id, auth_user.id, MessageSync::RoomCreate { room })
        .await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomUndelete { room_id },
    })
    .await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomUndelete { room_id },
    })
    .await?;

    Ok(())
}

/// Room audit logs
#[utoipa::path(
    get,
    path = "/room/{room_id}/audit-logs",
    params(
        PaginationQuery<AuditLogId>,
        ("room_id", description = "Room id"),
    ),
    tags = ["room", "badge.perm.ViewAuditLog"],
    responses(
        (status = 200, description = "fetch audit logs success", body = PaginationResponse<AuditLogEntry>),
    )
)]
async fn room_audit_logs(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<AuditLogEntryId>>,
    Auth(user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(user.id, room_id).await?;
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
    Auth(user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(user.id, room_id).await?;
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
    tags = ["room", "badge.sudo"],
    responses((status = OK, description = "success"))
)]
async fn room_transfer_ownership(
    Path(room_id): Path<RoomId>,
    AuthSudo(auth_user): AuthSudo,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<TransferOwnership>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();
    let target_user_id = json.owner_id;

    // ensure that target user is a room member
    data.room_member_get(room_id, target_user_id).await?;

    let perms = srv.perms.for_room(auth_user.id, room_id).await?;
    perms.ensure_view()?;
    let room_start = srv.rooms.get(room_id, Some(auth_user.id)).await?;
    if room_start.owner_id != Some(auth_user.id) {
        return Err(Error::BadStatic("you aren't the room owner"));
    }

    data.room_set_owner(room_id, target_user_id).await?;
    srv.perms.invalidate_room(auth_user.id, room_id).await;
    srv.perms.invalidate_room(target_user_id, room_id).await;
    srv.rooms.invalidate(room_id).await;
    let room = srv.rooms.get(room_id, Some(auth_user.id)).await?;
    let msg = MessageSync::RoomUpdate { room: room.clone() };
    s.broadcast_room(room_id, auth_user.id, msg).await?;
    Ok(Json(room))
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
struct TransferOwnership {
    owner_id: UserId,
}

/// Room integration list
///
/// list bots in a room
#[utoipa::path(
    get,
    path = "/room/{room_id}/integration",
    params(("room_id", description = "Room id")),
    tags = ["room"],
    responses((status = OK, description = "success", body = PaginationResponse<Integration>))
)]
async fn room_integration_list(
    Path(room_id): Path<RoomId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<PaginationQuery<ApplicationId>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_room(auth_user.id, room_id).await?;
    perms.ensure_view()?;
    let data = s.data();
    let ids = data.room_bot_list(room_id, q).await?;
    let mut integrations = vec![];
    for id in ids.items {
        let (app, bot, member) = tokio::join!(
            data.application_get(id),
            data.user_get(id.into_inner().into()),
            data.room_member_get(room_id, id.into_inner().into()),
        );
        integrations.push(Integration {
            application: app?,
            bot: bot?,
            member: member?,
        });
    }
    Ok(Json(PaginationResponse {
        items: integrations,
        total: ids.total,
        has_more: ids.has_more,
        cursor: ids.cursor,
    }))
}

/// Room quarantine
#[utoipa::path(
    post,
    path = "/room/{room_id}/quarantine",
    params(("room_id", description = "Room id")),
    tags = ["room", "badge.admin_only", "badge.perm.Admin"],
    responses((status = OK, description = "success"))
)]
async fn room_quarantine(
    Path(room_id): Path<RoomId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::Admin)?;

    let room = srv.rooms.get(room_id, None).await?;

    if room.quarantined {
        return Ok(Json(room));
    }

    data.room_quarantine(room_id).await?;
    srv.perms.invalidate_room_all(room_id);
    srv.rooms.invalidate(room_id).await;

    let updated_room = srv.rooms.get(room_id, None).await?;
    let msg = MessageSync::RoomUpdate {
        room: updated_room.clone(),
    };
    s.broadcast_room(room_id, auth_user.id, msg).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomQuarantine { room_id },
    })
    .await?;

    Ok(Json(updated_room))
}

/// Room unquarantine
#[utoipa::path(
    delete,
    path = "/room/{room_id}/quarantine",
    params(("room_id", description = "Room id")),
    tags = ["room", "badge.admin_only", "badge.perm.Admin"],
    responses((status = OK, description = "success"))
)]
async fn room_unquarantine(
    Path(room_id): Path<RoomId>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::Admin)?;

    let room = srv.rooms.get(room_id, None).await?;

    if !room.quarantined {
        return Ok(Json(room));
    }

    data.room_unquarantine(room_id).await?;
    srv.perms.invalidate_room_all(room_id);
    srv.rooms.invalidate(room_id).await;

    let updated_room = srv.rooms.get(room_id, None).await?;
    let msg = MessageSync::RoomUpdate {
        room: updated_room.clone(),
    };
    s.broadcast_room(room_id, auth_user.id, msg).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user.id,
        session_id: None,
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomUnquarantine { room_id },
    })
    .await?;

    Ok(Json(updated_room))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(room_create))
        .routes(routes!(room_get))
        .routes(routes!(room_list))
        .routes(routes!(room_edit))
        .routes(routes!(room_delete))
        .routes(routes!(room_undelete))
        .routes(routes!(room_audit_logs))
        .routes(routes!(room_ack))
        .routes(routes!(room_metrics))
        .routes(routes!(room_transfer_ownership))
        .routes(routes!(room_integration_list))
        .routes(routes!(room_quarantine))
        .routes(routes!(room_unquarantine))
}
