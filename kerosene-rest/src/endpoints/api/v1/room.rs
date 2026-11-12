use common::v1::types::{
    AuditLogEntryType, MessageSync, PaginationResponse, Permission, RoomType, SERVER_ROOM_ID,
    ack::{AckState, AckType},
    application::Scope,
    util::Changes,
};
use headers::{ETag, HeaderMapExt, LastModified};
use http::{HeaderMap, StatusCode};
use lamprey_backend_services::{
    compat::types::DbRoomCreate, services::search::SearchRoomsVisibility,
};

use crate::{prelude::*, util::request::ReqAuth};

fn cache_status(if_none_match: &Option<String>, version_id: &impl std::fmt::Display) -> StatusCode {
    let etag = format!(r#"W/"{version_id}""#);
    if if_none_match.as_ref().is_some_and(|val| val == &etag) {
        StatusCode::NOT_MODIFIED
    } else {
        StatusCode::OK
    }
}

#[handler(routes::room_create)]
pub async fn create(
    req: Req<routes::room_create::Endpoint>,
) -> Result<routes::room_create::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    body.room
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let srv = req.services();
    srv.perms
        .for_room3(Some(user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::RoomCreate)
        .check()?;

    if let Some(media_id) = body.room.icon {
        let media = req
            .globals()
            .begin_read()
            .await
            .cast_internal()?
            .media_select(media_id)
            .await
            .cast_internal()?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::MediaNotAnImage).into());
        }
    }

    let extra = DbRoomCreate {
        id: None,
        ty: RoomType::Default,
        welcome_channel_id: None,
        remote: None,
    };

    let mut auth = ReqAuth::new(&req);
    let handle = srv
        .rooms
        .create(
            body.room.clone(),
            &mut auth,
            extra,
            body.idempotency_key.clone(),
        )
        .await
        .cast_internal()?;
    let loaded = handle.ready(true).await.cast_internal()?;
    let room = (*loaded.room).clone();

    Ok(routes::room_create::Response { room })
}

#[handler(routes::room_get)]
pub async fn get(req: Req<routes::room_get::Endpoint>) -> Result<routes::room_get::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Rooms])?;

    let srv = req.services();
    let body = req.inner();
    let room = srv
        .rooms
        .get(body.room_id, identity.user_id())
        .await
        .cast_internal()?;
    if room.is_removed() {
        return Err(ApiError::from_code(ErrorCode::UnknownRoom).into());
    }

    srv.perms
        .for_room3(identity.user_id(), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let version_id = room.version_id;
    let status = cache_status(&body.if_none_match, &version_id);

    let mut headers = HeaderMap::new();
    let etag: ETag = format!(r#"W/"{version_id}""#).parse().unwrap();
    headers.typed_insert(etag);

    let timestamp: std::time::SystemTime = version_id
        .get_timestamp()
        .expect("all uuids are uuidv7")
        .into();
    headers.typed_insert(LastModified::from(timestamp));

    Ok(routes::room_get::Response {
        room,
        status,
        headers,
    })
}

#[handler(routes::room_list)]
pub async fn list(req: Req<routes::room_list::Endpoint>) -> Result<routes::room_list::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Rooms])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let is_admin = srv
        .perms
        .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .has(Permission::RoomManage);

    if !is_admin {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    let auth_user_id = auth_user.id;
    let body = req.into_inner();
    let mut data = globals.begin_read().await.cast_internal()?;
    let mut rooms = data.room_list_all(body.pagination).await.cast_internal()?;

    let mut new_rooms = Vec::with_capacity(rooms.items.len());
    for room in rooms.items {
        new_rooms.push(
            srv.rooms
                .get(room.id, Some(auth_user_id))
                .await
                .cast_internal()?,
        );
    }
    rooms.items = new_rooms;

    Ok(routes::room_list::Response { rooms })
}

#[handler(routes::room_search)]
pub async fn search(
    req: Req<routes::room_search::Endpoint>,
) -> Result<routes::room_search::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let srv = req.services();
    srv.perms
        .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::RoomManage)
        .check()?;

    let body = req.into_inner();
    let results = srv
        .search
        .search_rooms(SearchRoomsVisibility::Everything, body.search)
        .await
        .cast_internal()?;

    Ok(routes::room_search::Response { results })
}

#[handler(routes::room_edit)]
pub async fn edit(req: Req<routes::room_edit::Endpoint>) -> Result<routes::room_edit::Response> {
    let identity = req.identity();
    let user = identity.ensure_user()?;
    user.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let body = req.inner();
    body.patch
        .validate()
        .map_err(|e| ApiError::with_message(ErrorCode::InvalidData, e.to_string()))?;

    let srv = req.services();
    let room = srv
        .rooms
        .get(body.room_id, Some(user.id))
        .await
        .cast_internal()?;

    let changes_room_metadata = body.patch.name.as_ref().is_some_and(|p| p != &room.name)
        || body
            .patch
            .description
            .as_ref()
            .is_some_and(|p| p != &room.description)
        || body.patch.icon.as_ref().is_some_and(|p| p != &room.icon)
        || body
            .patch
            .banner
            .as_ref()
            .is_some_and(|p| p != &room.banner)
        || body
            .patch
            .public
            .as_ref()
            .is_some_and(|p| p != &room.public)
        || body
            .patch
            .welcome_channel_id
            .as_ref()
            .is_some_and(|p| p != &room.welcome_channel_id)
        || body
            .patch
            .afk_channel_id
            .as_ref()
            .is_some_and(|p| p != &room.afk_channel_id)
        || body
            .patch
            .afk_channel_timeout
            .as_ref()
            .is_some_and(|p| p != &room.afk_channel_timeout);
    let changes_invite_paused = body
        .patch
        .invites_paused_until
        .as_ref()
        .is_some_and(|p| p != &room.invites_paused_until);

    let mut perms = srv
        .perms
        .for_room3(Some(user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;

    if changes_room_metadata {
        perms.needs(Permission::RoomEdit);
    }
    if changes_invite_paused {
        perms.needs(Permission::InviteManage);
    }
    perms.check()?;

    if room.security.require_mfa {
        let mut data = req.globals().begin_read().await.cast_internal()?;
        let totp = data.auth_totp_get(user.id).await.cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
        }
    }

    if let Some(Some(media_id)) = body.patch.icon {
        let media = req
            .globals()
            .begin_read()
            .await
            .cast_internal()?
            .media_select(media_id)
            .await
            .cast_internal()?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::MediaNotAnImage).into());
        }
    }

    if let Some(Some(media_id)) = body.patch.banner {
        let media = req
            .globals()
            .begin_read()
            .await
            .cast_internal()?
            .media_select(media_id)
            .await
            .cast_internal()?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::MediaNotAnImage).into());
        }
    }

    let mut auth = ReqAuth::new(&req);
    let room = srv
        .rooms
        .update(body.room_id, &mut auth, body.patch.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_edit::Response { room })
}

#[handler(routes::room_delete)]
pub async fn delete(
    req: Req<routes::room_delete::Endpoint>,
) -> Result<routes::room_delete::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let perms = srv
        .perms
        .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?;
    let is_admin = perms.has(Permission::RoomManage);

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    if room.owner_id != Some(auth_user.id) && !is_admin {
        return Err(ApiError::from_code(ErrorCode::NotRoomOwner).into());
    }

    let mut data = globals.begin().await.cast_internal()?;
    data.room_delete(body.room_id).await.cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.rooms.invalidate(body.room_id).await;
    srv.perms.invalidate_room_all(body.room_id).await;

    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::RoomDelete {
                room_id: body.room_id,
            },
        )
        .await
        .cast_internal()?;

    let changes = Changes::new()
        .remove("name", &room.name)
        .remove("description", &room.description)
        .remove("icon", &room.icon)
        .remove("banner", &room.banner)
        .remove("public", &room.public)
        .build();

    let al = req.audit_log();
    al.push(
        body.room_id,
        AuditLogEntryType::RoomDelete {
            room_id: body.room_id,
            changes: changes.clone(),
        },
    )
    .success();
    al.push(
        SERVER_ROOM_ID.into_inner().into(),
        AuditLogEntryType::RoomDelete {
            room_id: body.room_id,
            changes,
        },
    )
    .success();

    Ok(routes::room_delete::Response {})
}

#[handler(routes::room_undelete)]
pub async fn undelete(
    req: Req<routes::room_undelete::Endpoint>,
) -> Result<routes::room_undelete::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::RoomManage)
        .check()?;

    let mut data = globals.begin().await.cast_internal()?;
    data.room_undelete(body.room_id).await.cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.rooms.reload(body.room_id).await.cast_internal()?;
    srv.perms.invalidate_room_all(body.room_id).await;

    let room = srv.rooms.get(body.room_id, None).await.cast_internal()?;
    globals
        .messaging()
        .broadcast_room(body.room_id, MessageSync::RoomCreate { room })
        .await
        .cast_internal()?;

    let al = req.audit_log();
    al.push(
        body.room_id,
        AuditLogEntryType::RoomUndelete {
            room_id: body.room_id,
        },
    )
    .success();
    al.push(
        SERVER_ROOM_ID.into_inner().into(),
        AuditLogEntryType::RoomUndelete {
            room_id: body.room_id,
        },
    )
    .success();

    Ok(routes::room_undelete::Response {})
}

#[handler(routes::room_audit_logs)]
pub async fn audit_logs(
    req: Req<routes::room_audit_logs::Endpoint>,
) -> Result<routes::room_audit_logs::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Rooms])?;

    let auth_user = identity.ensure_user()?;
    let srv = req.services();
    let body = req.inner();

    srv.perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::AuditLogView)
        .check()?;

    let logs = srv
        .audit_logs
        .list(body.room_id, body.pagination.clone(), body.filter.clone())
        .await
        .cast_internal()?;

    Ok(routes::room_audit_logs::Response { logs })
}

#[handler(routes::room_ack)]
pub async fn ack(req: Req<routes::room_ack::Endpoint>) -> Result<routes::room_ack::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Rooms])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let room_id = req.inner().room_id;

    srv.perms
        .for_room3(Some(auth_user.id), room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let acks = srv
        .read_states
        .ack_room(auth_user.id, room_id)
        .await
        .cast_internal()?;

    globals
        .messaging()
        .broadcast_user(
            auth_user.id,
            MessageSync::PassiveAck {
                user_id: auth_user.id,
                ack_states: acks
                    .into_iter()
                    .map(|(channel_id, message_id)| AckState {
                        ty: AckType::Message {
                            channel_id,
                            message_id,
                            mention_count: 0,
                        },
                        unread: false,
                    })
                    .collect(),
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::room_ack::Response {})
}

#[handler(routes::room_transfer_ownership)]
pub async fn transfer_ownership(
    req: Req<routes::room_transfer_ownership::Endpoint>,
) -> Result<routes::room_transfer_ownership::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;
    identity.ensure_sudo()?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();
    let target_user_id = body.transfer.owner_id;

    let mut data = globals.begin().await.cast_internal()?;
    data.room_member_get(body.room_id, target_user_id)
        .await
        .cast_internal()?;

    srv.perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?
        .check()?;

    let room_start = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    if room_start.owner_id != Some(auth_user.id) {
        return Err(ApiError::from_code(ErrorCode::NotRoomOwner).into());
    }

    data.room_set_owner(body.room_id, target_user_id)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.perms.invalidate_room(auth_user.id, body.room_id).await;
    srv.perms
        .invalidate_room(target_user_id, body.room_id)
        .await;
    srv.rooms.reload(body.room_id).await.cast_internal()?;

    let room = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;
    globals
        .messaging()
        .broadcast_room(body.room_id, MessageSync::RoomUpdate { room: room.clone() })
        .await
        .cast_internal()?;

    Ok(routes::room_transfer_ownership::Response { room })
}

#[handler(routes::room_integration_list)]
pub async fn integration_list(
    req: Req<routes::room_integration_list::Endpoint>,
) -> Result<routes::room_integration_list::Response> {
    let identity = req.identity();
    identity.ensure_scopes(&[Scope::Rooms])?;

    let auth_user = identity.ensure_user()?;
    let srv = req.services();
    let body = req.inner();

    let perms = srv
        .perms
        .for_room3(Some(auth_user.id), body.room_id)
        .await
        .cast_internal()?
        .ensure_view()?;
    perms.check()?;
    let is_moderator = perms.is_moderator();

    let mut data = req.globals().begin_read().await.cast_internal()?;
    let ids = data
        .room_bot_list(body.room_id, body.pagination.clone())
        .await
        .cast_internal()?;

    let mut integrations = Vec::with_capacity(ids.items.len());
    for id in ids.items {
        let application = data.application_get(id).await.cast_internal()?;
        let bot = data
            .user_get(id.into_inner().into())
            .await
            .cast_internal()?;
        let mut member = data
            .room_member_get(body.room_id, id.into_inner().into())
            .await
            .cast_internal()?;
        if !is_moderator {
            member.strip_private();
        }
        integrations.push(common::v1::types::application::Integration {
            application,
            bot,
            member,
        });
    }

    Ok(routes::room_integration_list::Response {
        integrations: PaginationResponse {
            items: integrations,
            total: ids.total,
            has_more: ids.has_more,
            cursor: ids.cursor,
        },
    })
}

#[handler(routes::room_quarantine)]
pub async fn quarantine(
    req: Req<routes::room_quarantine::Endpoint>,
) -> Result<routes::room_quarantine::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let room_id = req.inner().room_id;

    srv.perms
        .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::RoomManage)
        .check()?;

    let room = srv.rooms.get(room_id, None).await.cast_internal()?;
    if room.quarantined {
        return Ok(routes::room_quarantine::Response { room });
    }

    let mut data = globals.begin().await.cast_internal()?;
    data.room_quarantine(room_id).await.cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.perms.invalidate_room_all(room_id).await;
    srv.rooms.reload(room_id).await.cast_internal()?;

    let updated_room = srv.rooms.get(room_id, None).await.cast_internal()?;
    globals
        .messaging()
        .broadcast_room(
            room_id,
            MessageSync::RoomUpdate {
                room: updated_room.clone(),
            },
        )
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            SERVER_ROOM_ID.into_inner().into(),
            AuditLogEntryType::RoomQuarantine { room_id },
        )
        .success();

    Ok(routes::room_quarantine::Response { room: updated_room })
}

#[handler(routes::room_unquarantine)]
pub async fn unquarantine(
    req: Req<routes::room_unquarantine::Endpoint>,
) -> Result<routes::room_unquarantine::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let room_id = req.inner().room_id;

    srv.perms
        .for_room3(Some(auth_user.id), SERVER_ROOM_ID)
        .await
        .cast_internal()?
        .ensure_view()?
        .needs(Permission::RoomManage)
        .check()?;

    let room = srv.rooms.get(room_id, None).await.cast_internal()?;
    if !room.quarantined {
        return Ok(routes::room_unquarantine::Response { room });
    }

    let mut data = globals.begin().await.cast_internal()?;
    data.room_unquarantine(room_id).await.cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.perms.invalidate_room_all(room_id).await;
    srv.rooms.reload(room_id).await.cast_internal()?;

    let updated_room = srv.rooms.get(room_id, None).await.cast_internal()?;
    globals
        .messaging()
        .broadcast_room(
            room_id,
            MessageSync::RoomUpdate {
                room: updated_room.clone(),
            },
        )
        .await
        .cast_internal()?;

    req.audit_log()
        .push(
            SERVER_ROOM_ID.into_inner().into(),
            AuditLogEntryType::RoomUnquarantine { room_id },
        )
        .success();

    Ok(routes::room_unquarantine::Response { room: updated_room })
}

#[handler(routes::room_security_set)]
pub async fn security_set(
    req: Req<routes::room_security_set::Endpoint>,
) -> Result<routes::room_security_set::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;
    identity.ensure_sudo()?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let room = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    if room.owner_id != Some(auth_user.id) {
        return Err(ApiError::from_code(ErrorCode::MissingPermissions).into());
    }

    if body.security.require_mfa.is_none() && body.security.require_sudo.is_none() {
        return Ok(routes::room_security_set::Response { room });
    }

    if let Some(true) = body.security.require_mfa {
        let mut data = globals.begin_read().await.cast_internal()?;
        let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
        if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
            return Err(ApiError::from_code(ErrorCode::RoomOwnerMustHaveMfa).into());
        }
    }

    let start_security = room.security;

    let mut data = globals.begin().await.cast_internal()?;
    data.room_security_update(
        body.room_id,
        body.security.require_mfa,
        body.security.require_sudo,
    )
    .await
    .cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.rooms.reload(body.room_id).await.cast_internal()?;
    let room = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;

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
        req.audit_log()
            .push(body.room_id, AuditLogEntryType::RoomUpdate { changes })
            .success();
    }

    globals
        .messaging()
        .broadcast_room(body.room_id, MessageSync::RoomUpdate { room: room.clone() })
        .await
        .cast_internal()?;

    Ok(routes::room_security_set::Response { room })
}

#[handler(routes::room_feature_enable)]
pub async fn feature_enable(
    req: Req<routes::room_feature_enable::Endpoint>,
) -> Result<routes::room_feature_enable::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let perms_server = srv.perms.for_server(auth_user.id).await.cast_internal()?;
    let room_old = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    if perms_server.has(Permission::RoomManage) && body.feature.can_server_operators_enable() {
        // allowed
    } else {
        let mut perms_room = srv
            .perms
            .for_room3(Some(auth_user.id), body.room_id)
            .await
            .cast_internal()?
            .ensure_view()?;

        if body.feature.can_room_editors_enable() {
            perms_room.needs(Permission::RoomEdit).check()?;
        } else if body.feature.can_room_admins_enable() {
            perms_room.needs(Permission::Admin).check()?;
        } else {
            return Err(Error::internal_message(
                "feature cannot be manually enabled/disabled",
            ));
        }

        if room_old.security.require_mfa {
            let mut data = globals.begin_read().await.cast_internal()?;
            let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }
    }

    let mut features = room_old.features.0.clone();
    if features.contains(&body.feature) {
        return Ok(routes::room_feature_enable::Response { room: room_old });
    }
    features.push(body.feature);

    let mut data = globals.begin().await.cast_internal()?;
    data.room_set_features(body.room_id, &features)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.rooms.reload(body.room_id).await.cast_internal()?;
    let room_new = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    let changes = Changes::new()
        .change("features", &room_old.features, &room_new.features)
        .build();

    let al = req.audit_log();
    al.push(
        body.room_id,
        AuditLogEntryType::RoomUpdate {
            changes: changes.clone(),
        },
    )
    .success();
    al.push(
        SERVER_ROOM_ID.into_inner().into(),
        AuditLogEntryType::RoomUpdate { changes },
    )
    .success();

    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::RoomUpdate {
                room: room_new.clone(),
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::room_feature_enable::Response { room: room_new })
}

#[handler(routes::room_feature_disable)]
pub async fn feature_disable(
    req: Req<routes::room_feature_disable::Endpoint>,
) -> Result<routes::room_feature_disable::Response> {
    let identity = req.identity();
    identity.ensure_user()?.ensure_unsuspended()?;
    identity.ensure_scopes(&[Scope::Full])?;

    let auth_user = identity.ensure_user()?;
    let globals = req.globals();
    let srv = req.services();
    let body = req.inner();

    let perms_server = srv.perms.for_server(auth_user.id).await.cast_internal()?;
    let room_old = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    if perms_server.has(Permission::RoomManage) && body.feature.can_server_operators_enable() {
        // allowed
    } else {
        let mut perms_room = srv
            .perms
            .for_room3(Some(auth_user.id), body.room_id)
            .await
            .cast_internal()?
            .ensure_view()?;

        if body.feature.can_room_editors_enable() {
            perms_room.needs(Permission::RoomEdit).check()?;
        } else if body.feature.can_room_admins_enable() {
            perms_room.needs(Permission::Admin).check()?;
        } else {
            return Err(Error::internal_message(
                "feature cannot be manually enabled/disabled",
            ));
        }

        if room_old.security.require_mfa {
            let mut data = globals.begin_read().await.cast_internal()?;
            let totp = data.auth_totp_get(auth_user.id).await.cast_internal()?;
            if !totp.map(|(_, enabled)| enabled).unwrap_or(false) {
                return Err(ApiError::from_code(ErrorCode::MfaRequired).into());
            }
        }
    }

    let mut features = room_old.features.0.clone();
    if let Some(pos) = features.iter().position(|f| f == &body.feature) {
        features.remove(pos);
    } else {
        return Ok(routes::room_feature_disable::Response { room: room_old });
    }

    let mut data = globals.begin().await.cast_internal()?;
    data.room_set_features(body.room_id, &features)
        .await
        .cast_internal()?;
    data.commit().await.cast_internal()?;

    srv.rooms.reload(body.room_id).await.cast_internal()?;
    let room_new = srv
        .rooms
        .get(body.room_id, Some(auth_user.id))
        .await
        .cast_internal()?;

    let changes = Changes::new()
        .change("features", &room_old.features, &room_new.features)
        .build();

    let al = req.audit_log();
    al.push(
        body.room_id,
        AuditLogEntryType::RoomUpdate {
            changes: changes.clone(),
        },
    )
    .success();
    al.push(
        SERVER_ROOM_ID.into_inner().into(),
        AuditLogEntryType::RoomUpdate { changes },
    )
    .success();

    globals
        .messaging()
        .broadcast_room(
            body.room_id,
            MessageSync::RoomUpdate {
                room: room_new.clone(),
            },
        )
        .await
        .cast_internal()?;

    Ok(routes::room_feature_disable::Response { room: room_new })
}

export_routes!(
    create,
    get,
    list,
    search,
    edit,
    delete,
    undelete,
    audit_logs,
    ack,
    transfer_ownership,
    integration_list,
    quarantine,
    unquarantine,
    security_set,
    feature_enable,
    feature_disable
);
