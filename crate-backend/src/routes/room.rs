use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::TypedHeader;
use common::v1::types::{
    application::Integration, util::Changes, ApplicationId, AuditLogEntry, AuditLogEntryId,
    AuditLogEntryType, AuditLogFilter, RoomSecurityUpdate, RoomType, TransferOwnership,
    SERVER_ROOM_ID,
};
use headers::ETag;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{
    error::Result,
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RoomCreate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;

    // FIXME: run this in a transaction
    let icon = json.icon;
    if let Some(media_id) = icon {
        let data = s.data();
        let media = data.media_select(media_id).await?;
        if !matches!(
            media.inner.source.info,
            common::v1::types::MediaTrackInfo::Image(_)
        ) {
            return Err(Error::BadStatic("media not an image"));
        }
    }

    let extra = DbRoomCreate {
        id: None,
        ty: RoomType::Default,
        welcome_channel_id: None,
    };
    let room = s.services().rooms.create(json, auth.user.id, extra).await?;
    if let Some(media_id) = icon {
        let data = s.data();
        data.media_link_create_exclusive(media_id, *room.id, MediaLinkType::RoomIcon)
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
    auth: Auth,
    headers: HeaderMap,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let _perms = srv.perms.for_room(auth.user.id, room_id).await?;
    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;

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
/// Lists all rooms on the server.
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();
    let is_admin = srv
        .perms
        .for_room(auth.user.id, SERVER_ROOM_ID)
        .await?
        .has(Permission::Admin);

    if is_admin {
        let mut rooms = data.room_list_all(q).await?;

        let mut new_rooms = vec![];
        for room in rooms.items {
            new_rooms.push(srv.rooms.get(room.id, Some(auth.user.id)).await?);
        }
        rooms.items = new_rooms;

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
    tags = ["room", "badge.perm.RoomManage", "badge.room-sudo", "badge.room-mfa"],
    responses(
        (status = OK, description = "edit success"),
        (status = NOT_MODIFIED, description = "no change"),
    )
)]
async fn room_edit(
    Path((room_id,)): Path<(RoomId,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RoomPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    json.validate()?;
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::RoomManage)?;

    if let Some(Some(media_id)) = json.icon {
        let data = s.data();
        let media = data.media_select(media_id).await?;
        if !matches!(
            media.inner.source.info,
            common::v1::types::MediaTrackInfo::Image(_)
        ) {
            return Err(Error::BadStatic("media not an image"));
        }
    }

    let user_id = auth.user.id;

    let room = s
        .services()
        .rooms
        .update(room_id, auth, json.clone())
        .await?;

    if let Some(maybe_media_id) = json.icon {
        let data = s.data();
        data.media_link_delete(room_id.into_inner(), MediaLinkType::RoomIcon)
            .await?;
        if let Some(media_id) = maybe_media_id {
            data.media_link_create_exclusive(
                media_id,
                room_id.into_inner(),
                MediaLinkType::RoomIcon,
            )
            .await?;
        }
    }

    let msg = MessageSync::RoomUpdate { room: room.clone() };
    s.broadcast_room(room_id, user_id, msg).await?;
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_room(auth.user.id, SERVER_ROOM_ID).await?;
    let is_admin = perms.has(Permission::Admin);

    let room = srv.rooms.get(room_id, None).await?;
    if room.owner_id != Some(auth.user.id) && !is_admin {
        return Err(Error::BadStatic("you aren't the room owner"));
    }

    s.broadcast_room(room_id, auth.user.id, MessageSync::RoomDelete { room_id })
        .await?;

    data.room_delete(room_id).await?;
    srv.rooms.invalidate(room_id).await;
    srv.perms.invalidate_room_all(room_id).await;

    let changes = Changes::new()
        .remove("name", &room.name)
        .remove("description", &room.description)
        .remove("icon", &room.icon)
        .remove("public", &room.public)
        .build();

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomDelete {
            room_id,
            changes: changes.clone(),
        },
    })
    .await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomDelete { room_id, changes },
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_room(auth.user.id, SERVER_ROOM_ID).await?;
    perms.ensure(Permission::Admin)?;

    data.room_undelete(room_id).await?;
    srv.rooms.reload(room_id).await?;
    srv.perms.invalidate_room_all(room_id).await;

    let room = srv.rooms.get(room_id, None).await?;
    s.broadcast_room(room_id, auth.user.id, MessageSync::RoomCreate { room })
        .await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomUndelete { room_id },
    })
    .await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomUndelete { room_id },
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Room audit logs
#[utoipa::path(
    get,
    path = "/room/{room_id}/audit-logs",
    params(
        PaginationQuery<AuditLogEntryId>,
        AuditLogFilter,
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
    Query(filter): Query<AuditLogFilter>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = s.services().perms.for_room(auth.user.id, room_id).await?;
    perms.ensure(Permission::ViewAuditLog)?;
    let logs = data
        .audit_logs_room_fetch(room_id, paginate, filter)
        .await?;
    Ok(Json(logs))
}

/// Room ack
///
/// Mark all channels in a room as read.
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
    Path(room_id): Path<RoomId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    let data = s.data();
    let _perms = s.services().perms.for_room(auth.user.id, room_id).await?;

    let updated_unreads = data.unread_put_all_in_room(auth.user.id, room_id).await?;

    for (channel_id, message_id, version_id) in updated_unreads {
        s.broadcast(MessageSync::ChannelAck {
            user_id: auth.user.id,
            channel_id,
            message_id,
            version_id,
        })?;
    }

    Ok(Json(()))
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<TransferOwnership>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_sudo()?;

    let srv = s.services();
    let data = s.data();
    let target_user_id = json.owner_id;

    // ensure that target user is a room member
    data.room_member_get(room_id, target_user_id).await?;

    let _perms = srv.perms.for_room(auth.user.id, room_id).await?;
    let room_start = srv.rooms.get(room_id, Some(auth.user.id)).await?;
    if room_start.owner_id != Some(auth.user.id) {
        return Err(Error::BadStatic("you aren't the room owner"));
    }

    data.room_set_owner(room_id, target_user_id).await?;
    srv.perms.invalidate_room(auth.user.id, room_id).await;
    srv.perms.invalidate_room(target_user_id, room_id).await;
    srv.rooms.reload(room_id).await?;
    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;
    let msg = MessageSync::RoomUpdate { room: room.clone() };
    s.broadcast_room(room_id, auth.user.id, msg).await?;
    Ok(Json(room))
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<PaginationQuery<ApplicationId>>,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let _perms = srv.perms.for_room(auth.user.id, room_id).await?;
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_room(auth.user.id, SERVER_ROOM_ID).await?;
    perms.ensure(Permission::Admin)?;

    let room = srv.rooms.get(room_id, None).await?;

    if room.quarantined {
        return Ok(Json(room));
    }

    data.room_quarantine(room_id).await?;
    srv.perms.invalidate_room_all(room_id).await;
    srv.rooms.reload(room_id).await?;

    let updated_room = srv.rooms.get(room_id, None).await?;
    let msg = MessageSync::RoomUpdate {
        room: updated_room.clone(),
    };
    s.broadcast_room(room_id, auth.user.id, msg).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
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
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let srv = s.services();
    let data = s.data();

    let perms = srv.perms.for_room(auth.user.id, SERVER_ROOM_ID).await?;

    perms.ensure(Permission::Admin)?;

    let room = srv.rooms.get(room_id, None).await?;

    if !room.quarantined {
        return Ok(Json(room));
    }

    data.room_unquarantine(room_id).await?;
    srv.perms.invalidate_room_all(room_id).await;
    srv.rooms.reload(room_id).await?;

    let updated_room = srv.rooms.get(room_id, None).await?;
    let msg = MessageSync::RoomUpdate {
        room: updated_room.clone(),
    };
    s.broadcast_room(room_id, auth.user.id, msg).await?;

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth.user.id,
        session_id: None, // Note: Auth2 has session but this specific audit log doesn't use it
        reason: reason.clone(),
        ty: AuditLogEntryType::RoomUnquarantine { room_id },
    })
    .await?;

    Ok(Json(updated_room))
}

/// Room security set
#[utoipa::path(
    put,
    path = "/room/{room_id}/security",
    params(("room_id", description = "Room id")),
    request_body = RoomSecurityUpdate,
    tags = ["room", "badge.sudo"],
    responses(
        (status = OK, description = "success", body = Room),
    )
)]
async fn room_security_set(
    Path(room_id): Path<RoomId>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<RoomSecurityUpdate>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_sudo()?;

    let srv = s.services();
    let data = s.data();

    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;

    if room.owner_id != Some(auth.user.id) {
        return Err(Error::MissingPermissions);
    }

    if json.require_mfa.is_none() && json.require_sudo.is_none() {
        return Ok(Json(room));
    }

    if let Some(true) = json.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = data.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(Error::BadStatic("room owner must have mfa enabled"));
        }
    }

    let start_security = room.security;

    data.room_security_update(room_id, json.require_mfa, json.require_sudo)
        .await?;

    srv.rooms.reload(room_id).await?;
    let room = srv.rooms.get(room_id, Some(auth.user.id)).await?;

    let changes = Changes::new()
        .change(
            "require_mfa",
            &start_security.require_mfa,
            &room.security.require_mfa,
        )
        .change(
            "require_sudo",
            &start_security.require_sudo,
            &room.security.require_sudo,
        )
        .build();

    if !changes.is_empty() {
        s.audit_log_append(AuditLogEntry {
            id: AuditLogEntryId::new(),
            room_id,
            user_id: auth.user.id,
            session_id: Some(auth.session.id),
            reason,
            ty: AuditLogEntryType::RoomUpdate { changes },
        })
        .await?;
    }

    let msg = MessageSync::RoomUpdate { room: room.clone() };
    s.broadcast_room(room_id, auth.user.id, msg).await?;

    Ok(Json(room))
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
        .routes(routes!(room_transfer_ownership))
        .routes(routes!(room_integration_list))
        .routes(routes!(room_quarantine))
        .routes(routes!(room_unquarantine))
        .routes(routes!(room_security_set))
}
