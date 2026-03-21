use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use common::v1::routes;
use common::v1::types::application::Scope;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::harvest::{Harvest, HarvestCreate};
use common::v1::types::presence::Presence;
use common::v1::types::util::{Changes, Diff, Time};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MessageSync, PaginationQuery,
    PaginationResponse, Room, RoomId, SessionStatus, User, UserCreate, UserId, UserPatch,
    UserSearch, UserWithRelationship,
};
use common::v1::types::{
    AuditLogEntryStatus, AuditLogFilter, AuditLogPaginationResponse, HarvestId, Permission,
    SuspendRequest, Suspended, UserListParams, SERVER_ROOM_ID,
};
use http::StatusCode;
use lamprey_macros::handler;
use tracing::warn;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::routes::util::{Auth, AuthRelaxed2};
use crate::types::{DbUserCreate, MediaLinkType, RoomMemberPut, UserIdReq};
use crate::{routes2, ServerState};

use crate::error::{Error, Result};

/// User update
#[handler(routes::user_update)]
async fn user_update(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_update::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    auth.ensure_scopes(&[Scope::Full])?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_server(auth.user.id).await?;
    if auth.user.id != target_user_id {
        perms.ensure(Permission::UserManage)?;
    } else {
        perms.ensure(Permission::UserProfileSelf)?;
    }
    let data = s.data();
    let start = srv.users.get(target_user_id, Some(auth.user.id)).await?;
    if !req.patch.changes(&start) {
        return Ok(Json(start));
    }
    if let Some(Some(avatar_media_id)) = req.patch.avatar {
        let media = data.media_select(avatar_media_id).await?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::InvalidData).into());
        }
    }
    if let Some(Some(banner_media_id)) = req.patch.banner {
        let media = data.media_select(banner_media_id).await?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::InvalidData).into());
        }
    }
    data.user_update(target_user_id, req.patch.clone()).await?;
    if let Some(maybe_avatar) = req.patch.avatar {
        data.media_link_delete(target_user_id.into_inner(), MediaLinkType::UserAvatar)
            .await?;
        if let Some(avatar_media_id) = maybe_avatar {
            data.media_link_create_exclusive(
                avatar_media_id,
                target_user_id.into_inner(),
                MediaLinkType::UserAvatar,
            )
            .await?;
        }
    }
    if let Some(maybe_banner) = req.patch.banner {
        data.media_link_delete(target_user_id.into_inner(), MediaLinkType::UserBanner)
            .await?;
        if let Some(banner_media_id) = maybe_banner {
            data.media_link_create_exclusive(
                banner_media_id,
                target_user_id.into_inner(),
                MediaLinkType::UserBanner,
            )
            .await?;
        }
    }
    srv.users.invalidate(target_user_id).await;
    let user = srv.users.get(target_user_id, Some(auth.user.id)).await?;
    let changes = Changes::new()
        .change("name", &start.name, &user.name)
        .change("description", &start.description, &user.description)
        .change("avatar", &start.avatar, &user.avatar)
        .change("banner", &start.banner, &user.banner)
        .build();

    let al = auth.audit_log(target_user_id.into_inner().into());
    al.commit_success(AuditLogEntryType::UserUpdate {
        changes: changes.clone(),
    })
    .await?;

    if auth.user.id != target_user_id {
        let al = auth.audit_log(SERVER_ROOM_ID);
        al.commit_success(AuditLogEntryType::UserUpdate { changes })
            .await?;
    }

    s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;
    Ok(Json(user))
}

/// User delete
#[handler(routes::user_delete)]
async fn user_delete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_delete::Request,
) -> Result<impl IntoResponse> {
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_server(auth.user.id).await?;
    if auth.user.id != target_user_id {
        perms.ensure(Permission::UserManage)?;
    } else {
        perms.ensure(Permission::UserManageSelf)?;
    }

    let user_to_delete = srv.users.get(target_user_id, Some(auth.user.id)).await?;
    data.user_delete(target_user_id).await?;
    data.media_link_delete(target_user_id.into_inner(), MediaLinkType::UserAvatar)
        .await?;
    let srv = s.services();
    srv.users.invalidate(target_user_id).await;
    s.broadcast(MessageSync::UserDelete { id: target_user_id })?;
    let al = auth.audit_log(target_user_id.into_inner().into());
    al.commit_success(AuditLogEntryType::UserDelete {
        user_id: target_user_id,
        changes: Changes::new()
            .remove("name", &user_to_delete.name)
            .remove("description", &user_to_delete.description)
            .remove("avatar", &user_to_delete.avatar)
            .remove("banner", &user_to_delete.banner)
            .build(),
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// User undelete
///
/// Allows undeleting a user provided they haven't been garbage collected yet
#[handler(routes::user_undelete)]
async fn user_undelete(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_undelete::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::UserManage)?;

    data.user_undelete(target_user_id).await?;

    let user = srv.users.get(target_user_id, Some(auth.user.id)).await?;
    let avatar_media_id = user.avatar;
    if let Some(media_id) = avatar_media_id {
        if data
            .media_link_create_exclusive(
                media_id,
                target_user_id.into_inner(),
                MediaLinkType::UserAvatar,
            )
            .await
            .is_err()
        {
            warn!("failed to re-link avatar for user {}", target_user_id);
            data.user_update(
                target_user_id,
                UserPatch {
                    avatar: Some(None),
                    ..Default::default()
                },
            )
            .await?;
        }
    }

    srv.users.invalidate(target_user_id).await;
    let user = srv.users.get(target_user_id, Some(auth.user.id)).await?;
    s.broadcast(MessageSync::UserCreate { user: user.clone() })?;

    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::UserUndelete {
        user_id: target_user_id,
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// User get
#[handler(routes::user_get)]
async fn user_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_get::Request,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Identify])?;
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let data = s.data();
    let mut user = srv.users.get(target_user_id, Some(auth.user.id)).await?;
    if !auth.scopes.iter().any(|s| s.implies(&Scope::Email)) {
        user.emails = None;
    }
    let relationship = data
        .user_relationship_get(auth.user.id, target_user_id)
        .await?
        .unwrap_or_default();
    Ok(Json(UserWithRelationship {
        inner: user,
        relationship,
    }))
}

/// User rooms list
///
/// List rooms a user is in. If you are not the user, lists mutual rooms.
#[handler(routes::user_room_list)]
async fn user_room_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_room_list::Request,
) -> Result<impl IntoResponse> {
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let data = s.data();
    let srv = s.services();
    let mut rooms = if auth.user.id == target_user_id {
        data.room_list(auth.user.id, req.pagination, false).await?
    } else {
        data.room_list_mutual(auth.user.id, target_user_id, req.pagination)
            .await?
    };

    let mut new_rooms = vec![];
    for room in rooms.items {
        new_rooms.push(srv.rooms.get(room.id, Some(auth.user.id)).await?);
    }
    rooms.items = new_rooms;

    Ok(Json(rooms))
}

/// User audit logs
#[handler(routes::user_audit_logs)]
async fn user_audit_logs(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_audit_logs::Request,
) -> Result<impl IntoResponse> {
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownUser)));
    }

    let logs = s
        .services()
        .audit_logs
        .list(
            target_user_id.into_inner().into(),
            req.pagination,
            req.filter,
        )
        .await?;
    Ok(Json(logs))
}

/// Guest create
///
/// Create a guest account, with limited access to the platform.
///
/// - guests can read but not write public rooms, threads, messages, etc
/// - when using an invite, they can act like a standard account in that one specific room/thread
/// - they can be given an invite to a public room to bypass
#[handler(routes::guest_create)]
async fn guest_create(
    auth: AuthRelaxed2,
    State(s): State<Arc<ServerState>>,
    req: routes::guest_create::Request,
) -> Result<impl IntoResponse> {
    let session = auth.session;
    let data = s.data();
    let srv = s.services();

    let user = data
        .user_create(DbUserCreate {
            id: None,
            parent_id: None,
            name: req.create.name.clone(),
            description: req.create.description.clone(),
            puppet: None,
            registered_at: if s.config.require_server_invite {
                None
            } else {
                Some(Time::now_utc())
            },
            system: false,
        })
        .await?;

    data.room_member_put(SERVER_ROOM_ID, user.id, None, RoomMemberPut::default())
        .await?;
    srv.perms.invalidate_room(user.id, SERVER_ROOM_ID).await;

    data.session_set_status(session.id, SessionStatus::Authorized { user_id: user.id })
        .await?;
    srv.sessions.invalidate(session.id).await;
    let updated_session = srv.sessions.get(session.id).await?;
    s.broadcast(MessageSync::SessionCreate {
        session: updated_session.clone(),
    })?;

    let entry = AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: (*user.id).into(),
        user_id: user.id,
        session_id: Some(updated_session.id),
        reason: None,
        ty: AuditLogEntryType::SessionLogin {
            user_id: user.id,
            session_id: updated_session.id,
        },
        status: AuditLogEntryStatus::Success,
        started_at: updated_session.authorized_at.unwrap_or_else(Time::now_utc),
        ended_at: Time::now_utc(),
        ip_addr: updated_session.ip_addr.clone(),
        user_agent: updated_session.user_agent.clone(),
        application_id: updated_session.app_id,
    };
    data.audit_logs_room_append(entry.clone()).await?;
    s.broadcast_room(
        entry.room_id,
        entry.user_id,
        MessageSync::AuditLogEntryCreate { entry },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(user)))
}

/// User suspend
#[handler(routes::user_suspend)]
async fn user_suspend(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_suspend::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let d = s.data();
    let srv = s.services();
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if target_user_id != auth.user.id {
        let perms = srv.perms.for_server(auth.user.id).await?;
        perms.ensure(Permission::MemberBan)?;
    }
    d.user_suspended(
        target_user_id,
        Some(Suspended {
            created_at: Time::now_utc(),
            expires_at: req.suspend.expires_at,
            reason: req.reason.clone(),
        }),
    )
    .await?;
    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::UserSuspend {
        expires_at: req.suspend.expires_at,
        user_id: target_user_id,
    })
    .await?;
    srv.users.invalidate(target_user_id).await;
    let user = srv.users.get(target_user_id, Some(auth.user.id)).await?;
    s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;
    Ok(Json(user))
}

/// User unsuspend
#[handler(routes::user_unsuspend)]
async fn user_unsuspend(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_unsuspend::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let d = s.data();
    let srv = s.services();
    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::MemberBan)?;
    d.user_suspended(target_user_id, None).await?;
    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::UserUnsuspend {
        user_id: target_user_id,
    })
    .await?;
    srv.users.invalidate(target_user_id).await;
    let user = srv.users.get(target_user_id, Some(auth.user.id)).await?;
    s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;
    Ok(Json(user))
}

/// User presence set
///
/// for puppets
#[handler(routes::user_presence_set)]
async fn user_presence_set(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_presence_set::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let target_user_id = match req.user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    let srv = s.services();
    srv.presence
        .set_manual(target_user_id, req.presence)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// User list
///
/// Admin only. List all users on this server.
// TODO: deprecate
#[handler(routes::user_list)]
async fn user_list(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: routes::user_list::Request,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::MemberBan)?;

    let data = s.data();
    let mut users = data.user_list(req.pagination, req.filter).await?;

    for user in &mut users.items {
        user.emails = Some(data.user_email_list(user.id).await?);
    }

    Ok(Json(users))
}

/// Harvest get
#[handler(routes::harvest_get)]
async fn harvest_get(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::harvest_get::Request,
) -> Result<impl IntoResponse> {
    if auth.user.bot || auth.user.webhook.is_some() || auth.user.puppet.is_some() {
        return Err(ApiError::from_code(ErrorCode::BotsCannotUseThisEndpoint).into());
    }
    auth.user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Harvest create
#[handler(routes::harvest_create)]
async fn harvest_create(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    _req: routes::harvest_create::Request,
) -> Result<impl IntoResponse> {
    if auth.user.bot || auth.user.webhook.is_some() || auth.user.puppet.is_some() {
        return Err(ApiError::from_code(ErrorCode::BotsCannotUseThisEndpoint).into());
    }
    auth.user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Harvest download
#[handler(routes::harvest_download)]
async fn harvest_download(
    State(_s): State<Arc<ServerState>>,
    _req: routes::harvest_download::Request,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

/// User search (TODO)
#[handler(routes::user_search)]
async fn user_search(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    _req: routes::user_search::Request,
) -> Result<impl IntoResponse> {
    let srv = s.services();
    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::Admin)?;

    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes2!(user_update))
        .routes(routes2!(user_get))
        .routes(routes2!(user_delete))
        .routes(routes2!(user_undelete))
        .routes(routes2!(user_audit_logs))
        .routes(routes2!(user_room_list))
        .routes(routes2!(user_suspend))
        .routes(routes2!(user_unsuspend))
        .routes(routes2!(guest_create))
        .routes(routes2!(user_presence_set))
        .routes(routes2!(user_list))
        .routes(routes2!(harvest_get))
        .routes(routes2!(harvest_create))
        .routes(routes2!(harvest_download))
        .routes(routes2!(user_search))
}
