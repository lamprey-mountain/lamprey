use std::sync::Arc;
use std::time::{Duration, SystemTime};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::misc::time::Time;
use common::v1::types::util::Changes;
use common::v1::types::{AuditLogEntryType, RoomType, SERVER_ROOM_ID};
use http::header::{HeaderMap, HeaderName, HeaderValue};
use lamprey_macros::handler;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;
use validator::Validate;

use crate::routes::util::{Auth, AuthRelaxed2};
use crate::routes2;
use crate::types::{DbRoomCreate, MediaLinkType, MessageSync, PaginationResponse, Permission};
use crate::{error::Result, Error, ServerState};
use common::v1::types::error::{ApiError, ErrorCode};

fn build_cache_headers(version_id: &Uuid) -> Result<HeaderMap> {
    let ts: Time = version_id
        .get_timestamp()
        .expect("this is a uuid v7")
        .try_into()
        .expect("uuids are always valid timestamps");
    let etag = format!(r#"W/"{}""#, version_id);
    let headers = HeaderMap::from_iter([
        (
            HeaderName::from_static("last-modified"),
            HeaderValue::from_str(&httpdate::fmt_http_date(
                (SystemTime::UNIX_EPOCH
                    + Duration::from_nanos(ts.unix_timestamp_nanos().try_into().unwrap_or(0)))
                .into(),
            ))
            .unwrap(),
        ),
        (
            HeaderName::from_static("etag"),
            HeaderValue::from_str(&etag).unwrap(),
        ),
    ]);
    Ok(headers)
}

fn check_cache(if_none_match: &Option<String>, version_id: &Uuid) -> Result<()> {
    if let Some(val) = if_none_match {
        let etag = format!(r#"W/"{}""#, version_id);
        if val == &etag {
            return Err(Error::NotModified);
        }
    }
    Ok(())
}

/// Room create
#[handler(routes::room_create)]
async fn room_create(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_create::Request,
) -> Result<impl IntoResponse> {
    tracing::debug!("room_create for user: {:?}", auth.user.id);
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.room.validate()?;

    let srv = s.services();
    let perms = srv
        .perms
        .for_room3(Some(auth.user.id), SERVER_ROOM_ID)
        .await?
        .ensure_view()?
        .needs(Permission::RoomCreate)
        .check()?;

    tracing::debug!("server perms for {}: {:?}", auth.user.id, perms);

    let icon = req.room.icon;
    if let Some(media_id) = icon {
        let data = s.data();
        let media = data.media_select(media_id).await?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::MediaNotAnImage).into());
        }
    }

    let extra = DbRoomCreate {
        id: None,
        ty: RoomType::Default,
        welcome_channel_id: None,
    };
    let room = srv
        .rooms
        .create(req.room, &auth, extra, req.idempotency_key)
        .await?;
    if let Some(media_id) = icon {
        let data = s.data();
        data.media_link_create_exclusive(media_id, *room.id, MediaLinkType::RoomIcon)
            .await?;
    }

    Ok((StatusCode::CREATED, Json(room)))
}

/// Room get
#[handler(routes::room_get)]
async fn room_get(
    auth: AuthRelaxed2,
    State(s): State<Arc<ServerState>>,
    req: routes::room_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Rooms])?;
    let srv = s.services();

    let user_id = auth.user.as_ref().map(|u| u.id);

    let room = srv.rooms.get(req.room_id, user_id).await?;
    if room.is_removed() {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownRoom)));
    }

    srv.perms
        .for_room3(user_id, req.room_id)
        .await?
        .ensure_view()?
        .check()?;

    check_cache(&req.if_none_match, &room.version_id)?;
    let headers = build_cache_headers(&room.version_id)?;
    Ok((headers, Json(room)))
}

/// Room list
#[handler(routes::room_list)]
async fn room_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Rooms])?;
    let data = s.data();
    let srv = s.services();
    let is_admin = srv
        .perms
        .for_room3(Some(auth.user.id), SERVER_ROOM_ID)
        .await?
        .has(Permission::RoomManage);

    if is_admin {
        let mut rooms = data.room_list_all(req.pagination).await?;

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

/// Room search (TODO)
#[handler(routes::room_search)]
async fn room_search(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::room_search::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Full])?;
    let srv = s.services();
    srv.perms
        .for_room3(Some(auth.user.id), SERVER_ROOM_ID)
        .await?
        .ensure_view()?
        .needs(Permission::RoomManage)
        .check()?;

    Ok(Error::Unimplemented)
}

/// Room edit
#[handler(routes::room_edit)]
async fn room_edit(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_edit::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    req.patch.validate()?;
    let srv = s.services();
    srv.perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?
        .needs(Permission::RoomEdit)
        .check()?;

    let room = s
        .services()
        .rooms
        .get(req.room_id, Some(auth.user.id))
        .await?;
    if room.security.require_mfa {
        let data = s.data();
        let totp = data.auth_totp_get(auth.user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    if let Some(Some(media_id)) = req.patch.icon {
        let data = s.data();
        let media = data.media_select(media_id).await?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::MediaNotAnImage).into());
        }
    }

    let user_id = auth.user.id;

    let room = s
        .services()
        .rooms
        .update(req.room_id, auth, req.patch.clone())
        .await?;

    if let Some(maybe_media_id) = req.patch.icon {
        let data = s.data();
        data.media_link_delete(req.room_id.into_inner(), MediaLinkType::RoomIcon)
            .await?;
        if let Some(media_id) = maybe_media_id {
            data.media_link_create_exclusive(
                media_id,
                req.room_id.into_inner(),
                MediaLinkType::RoomIcon,
            )
            .await?;
        }
    }

    let msg = MessageSync::RoomUpdate { room: room.clone() };
    s.broadcast_room(req.room_id, user_id, msg).await?;
    Ok(Json(room))
}

/// Room delete
#[handler(routes::room_delete)]
async fn room_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_delete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let data = s.data();

    let perms = srv
        .perms
        .for_room3(Some(auth.user.id), SERVER_ROOM_ID)
        .await?
        .ensure_view()?;
    let is_admin = perms.has(Permission::RoomManage);

    let room = srv.rooms.get(req.room_id, None).await?;
    if room.owner_id != Some(auth.user.id) && !is_admin {
        return Err(ApiError::from_code(ErrorCode::NotRoomOwner).into());
    }

    s.broadcast_room(
        req.room_id,
        auth.user.id,
        MessageSync::RoomDelete {
            room_id: req.room_id,
        },
    )
    .await?;

    data.room_delete(req.room_id).await?;
    srv.rooms.invalidate(req.room_id).await;
    srv.perms.invalidate_room_all(req.room_id).await;

    let changes = Changes::new()
        .remove("name", &room.name)
        .remove("description", &room.description)
        .remove("icon", &room.icon)
        .remove("banner", &room.banner)
        .remove("public", &room.public)
        .build();

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::RoomDelete {
        room_id: req.room_id,
        changes: changes.clone(),
    })
    .await?;

    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::RoomDelete {
        room_id: req.room_id,
        changes,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Room undelete
#[handler(routes::room_undelete)]
async fn room_undelete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_undelete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let data = s.data();

    srv.perms
        .for_room3(Some(auth.user.id), SERVER_ROOM_ID)
        .await?
        .ensure_view()?
        .needs(Permission::RoomManage)
        .check()?;

    data.room_undelete(req.room_id).await?;
    srv.rooms.reload(req.room_id).await?;
    srv.perms.invalidate_room_all(req.room_id).await;

    let room = srv.rooms.get(req.room_id, None).await?;
    s.broadcast_room(req.room_id, auth.user.id, MessageSync::RoomCreate { room })
        .await?;

    let al = auth.audit_log(req.room_id);
    al.commit_success(AuditLogEntryType::RoomUndelete {
        room_id: req.room_id,
    })
    .await?;

    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::RoomUndelete {
        room_id: req.room_id,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Room audit logs
#[handler(routes::room_audit_logs)]
async fn room_audit_logs(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_audit_logs::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Rooms])?;
    let srv = s.services();
    srv.perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?
        .needs(Permission::AuditLogView)
        .check()?;
    let logs = s
        .services()
        .audit_logs
        .list(req.room_id, req.pagination, req.filter)
        .await?;
    Ok(Json(logs))
}

/// Room ack
#[handler(routes::room_ack)]
async fn room_ack(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_ack::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Rooms])?;
    let data = s.data();
    let srv = s.services();
    srv.perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?
        .check()?;

    let updated_unreads = data
        .unread_put_all_in_room(auth.user.id, req.room_id)
        .await?;

    for (channel_id, message_id, version_id) in updated_unreads {
        s.broadcast(MessageSync::ChannelAck {
            user_id: auth.user.id,
            channel_id,
            message_id,
            version_id,
        })?;
    }

    Ok(StatusCode::OK)
}

/// Room transfer ownership
#[handler(routes::room_transfer_ownership)]
async fn room_transfer_ownership(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_transfer_ownership::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    auth.ensure_sudo()?;

    let srv = s.services();
    let data = s.data();
    let target_user_id = req.transfer.owner_id;

    data.room_member_get(req.room_id, target_user_id).await?;

    srv.perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?
        .check()?;
    let room_start = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;
    if room_start.owner_id != Some(auth.user.id) {
        return Err(ApiError::from_code(ErrorCode::NotRoomOwner).into());
    }

    data.room_set_owner(req.room_id, target_user_id).await?;
    srv.perms.invalidate_room(auth.user.id, req.room_id).await;
    srv.perms.invalidate_room(target_user_id, req.room_id).await;
    srv.rooms.reload(req.room_id).await?;
    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;
    let msg = MessageSync::RoomUpdate { room: room.clone() };
    s.broadcast_room(req.room_id, auth.user.id, msg).await?;
    Ok(Json(room))
}

/// Room integration list
#[handler(routes::room_integration_list)]
async fn room_integration_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_integration_list::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Rooms])?;
    let srv = s.services();
    srv.perms
        .for_room3(Some(auth.user.id), req.room_id)
        .await?
        .ensure_view()?
        .check()?;
    let data = s.data();
    let ids = data.room_bot_list(req.room_id, req.pagination).await?;
    let mut integrations = vec![];
    for id in ids.items {
        let (app, bot, member) = tokio::join!(
            data.application_get(id),
            data.user_get(id.into_inner().into()),
            data.room_member_get(req.room_id, id.into_inner().into()),
        );
        integrations.push(common::v1::types::application::Integration {
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
#[handler(routes::room_quarantine)]
async fn room_quarantine(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_quarantine::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let data = s.data();

    srv.perms
        .for_room3(Some(auth.user.id), SERVER_ROOM_ID)
        .await?
        .ensure_view()?
        .needs(Permission::RoomManage)
        .check()?;

    let room = srv.rooms.get(req.room_id, None).await?;

    if room.quarantined {
        return Ok(Json(room));
    }

    data.room_quarantine(req.room_id).await?;
    srv.perms.invalidate_room_all(req.room_id).await;
    srv.rooms.reload(req.room_id).await?;

    let updated_room = srv.rooms.get(req.room_id, None).await?;
    let msg = MessageSync::RoomUpdate {
        room: updated_room.clone(),
    };
    s.broadcast_room(req.room_id, auth.user.id, msg).await?;

    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::RoomQuarantine {
        room_id: req.room_id,
    })
    .await?;

    Ok(Json(updated_room))
}

/// Room unquarantine
#[handler(routes::room_unquarantine)]
async fn room_unquarantine(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_unquarantine::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;

    let srv = s.services();
    let data = s.data();

    srv.perms
        .for_room3(Some(auth.user.id), SERVER_ROOM_ID)
        .await?
        .ensure_view()?
        .needs(Permission::RoomManage)
        .check()?;

    let room = srv.rooms.get(req.room_id, None).await?;

    if !room.quarantined {
        return Ok(Json(room));
    }

    data.room_unquarantine(req.room_id).await?;
    srv.perms.invalidate_room_all(req.room_id).await;
    srv.rooms.reload(req.room_id).await?;

    let updated_room = srv.rooms.get(req.room_id, None).await?;
    let msg = MessageSync::RoomUpdate {
        room: updated_room.clone(),
    };
    s.broadcast_room(req.room_id, auth.user.id, msg).await?;

    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::RoomUnquarantine {
        room_id: req.room_id,
    })
    .await?;

    Ok(Json(updated_room))
}

/// Room security set
#[handler(routes::room_security_set)]
async fn room_security_set(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::room_security_set::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    auth.ensure_sudo()?;

    let srv = s.services();
    let data = s.data();

    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;

    if room.owner_id != Some(auth.user.id) {
        return Err(Error::MissingPermissions);
    }

    if req.security.require_mfa.is_none() && req.security.require_sudo.is_none() {
        return Ok(Json(room));
    }

    if let Some(true) = req.security.require_mfa {
        let user = srv.users.get(auth.user.id, None).await?;
        let totp = data.auth_totp_get(user.id).await?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::RoomOwnerMustHaveMfa).into());
        }
    }

    let start_security = room.security;

    data.room_security_update(
        req.room_id,
        req.security.require_mfa,
        req.security.require_sudo,
    )
    .await?;

    srv.rooms.reload(req.room_id).await?;
    let room = srv.rooms.get(req.room_id, Some(auth.user.id)).await?;

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
        let al = auth.audit_log(req.room_id);
        al.commit_success(AuditLogEntryType::RoomUpdate { changes })
            .await?;
    }

    let msg = MessageSync::RoomUpdate { room: room.clone() };
    s.broadcast_room(req.room_id, auth.user.id, msg).await?;

    Ok(Json(room))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(room_create))
        .routes(routes2!(room_get))
        .routes(routes2!(room_list))
        .routes(routes2!(room_search))
        .routes(routes2!(room_edit))
        .routes(routes2!(room_delete))
        .routes(routes2!(room_undelete))
        .routes(routes2!(room_audit_logs))
        .routes(routes2!(room_ack))
        .routes(routes2!(room_transfer_ownership))
        .routes(routes2!(room_integration_list))
        .routes(routes2!(room_quarantine))
        .routes(routes2!(room_unquarantine))
        .routes(routes2!(room_security_set))
}
