use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;
use common::unstable::types::harvest::{Harvest, HarvestCreate};
use common::v1::types::application::Scope;
use common::v1::types::presence::Presence;
use common::v1::types::util::{Changes, Diff, Time};
use common::v1::types::{
    AuditLogEntry, AuditLogEntryId, AuditLogEntryType, MediaTrackInfo, MessageSync,
    PaginationQuery, PaginationResponse, Room, RoomId, SessionStatus, User, UserCreate, UserId,
    UserPatch, UserWithRelationship,
};
use common::v1::types::{
    AuditLogEntryStatus, AuditLogFilter, HarvestId, Permission, SuspendRequest, Suspended,
    UserListParams, SERVER_ROOM_ID,
};
use http::StatusCode;
use tracing::warn;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::routes::util::{Auth, HeaderReason};
use crate::types::{DbUserCreate, MediaLinkType, UserIdReq};
use crate::ServerState;

use super::util::AuthRelaxed;
use crate::error::{Error, Result};

/// User update
#[utoipa::path(
    patch,
    path = "/user/{user_id}",
    params(
        ("user_id", description = "User id"),
    ),
    tags = ["user"],
    responses(
        (status = OK, body = User, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn user_update(
    Path(target_user_id): Path<UserIdReq>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<UserPatch>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let perms = srv.perms.for_server(auth.user.id).await?;
    if auth.user.id != target_user_id {
        perms.ensure(Permission::UserManage)?;
    } else {
        perms.ensure(Permission::UserProfile)?;
    }
    let data = s.data();
    let start = srv.users.get(target_user_id, Some(auth.user.id)).await?;
    if !patch.changes(&start) {
        return Ok(Json(start));
    }
    if let Some(Some(avatar_media_id)) = patch.avatar {
        let media = data.media_select(avatar_media_id).await?;
        if !matches!(media.inner.source.info, MediaTrackInfo::Image(_)) {
            return Err(Error::BadStatic(
                "couldn't link media as avatar: not an image",
            ));
        }
    }
    if let Some(Some(banner_media_id)) = patch.banner {
        let media = data.media_select(banner_media_id).await?;
        if !matches!(media.inner.source.info, MediaTrackInfo::Image(_)) {
            return Err(Error::BadStatic(
                "couldn't link media as banner: not an image",
            ));
        }
    }
    data.user_update(target_user_id, patch.clone()).await?;
    if let Some(maybe_avatar) = patch.avatar {
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
    if let Some(maybe_banner) = patch.banner {
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
#[utoipa::path(
    delete,
    path = "/user/{user_id}",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn user_delete(
    Path(target_user_id): Path<UserIdReq>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let data = s.data();
    let perms = srv.perms.for_server(auth.user.id).await?;
    if auth.user.id != target_user_id {
        perms.ensure(Permission::UserManage)?;
    } else {
        perms.ensure(Permission::UserDeleteSelf)?;
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
#[utoipa::path(
    post,
    path = "/user/{user_id}/undelete",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn user_undelete(
    Path(target_user_id): Path<UserIdReq>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let target_user_id = match target_user_id {
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
///
/// Get another user, including your relationship
#[utoipa::path(
    get,
    path = "/user/{user_id}",
    params(
        ("user_id", description = "User id"),
    ),
    tags = ["user", "badge.scope.identify", "badge.scope-opt.email"],
    responses(
        (status = OK, body = UserWithRelationship, description = "success"),
    )
)]
async fn user_get(
    Path(target_user_id): Path<UserIdReq>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.ensure_scopes(&[Scope::Identify])?;
    let target_user_id = match target_user_id {
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
#[utoipa::path(
    get,
    path = "/user/{user_id}/room",
    params(
        PaginationQuery<RoomId>,
        ("user_id", description = "user id"),
    ),
    tags = ["user"],
    responses(
        (status = OK, body = PaginationResponse<Room>, description = "success"),
    )
)]
async fn user_room_list(
    Path(target_user_id): Path<UserIdReq>,
    auth: Auth,
    Query(q): Query<PaginationQuery<RoomId>>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    let data = s.data();
    let srv = s.services();
    let mut rooms = if auth.user.id == target_user_id {
        data.room_list(auth.user.id, q, false).await?
    } else {
        data.room_list_mutual(auth.user.id, target_user_id, q)
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
#[utoipa::path(
    get,
    path = "/user/{user_id}/audit-logs",
    params(
        PaginationQuery<AuditLogEntryId>,
        AuditLogFilter,
        ("user_id", description = "User id"),
    ),
    tags = ["user"],
    responses(
        (status = OK, body = PaginationResponse<AuditLogEntry>, description = "success"),
    )
)]
async fn user_audit_logs(
    Path(target_user_id): Path<UserIdReq>,
    Query(paginate): Query<PaginationQuery<AuditLogEntryId>>,
    Query(filter): Query<AuditLogFilter>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::NotFound);
    }

    let data = s.data();
    let logs = data
        .audit_logs_room_fetch(target_user_id.into_inner().into(), paginate, filter)
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
#[utoipa::path(
    post,
    path = "/guest",
    tags = ["user"],
    responses((status = CREATED, body = User, description = "guest account created")),
)]
async fn guest_create(
    AuthRelaxed(session): AuthRelaxed,
    State(s): State<Arc<ServerState>>,
    Json(create): Json<UserCreate>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let srv = s.services();

    let user = data
        .user_create(DbUserCreate {
            id: None,
            parent_id: None,
            name: create.name,
            description: create.description,
            puppet: None,
            registered_at: None,
            system: false,
        })
        .await?;

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
#[utoipa::path(
    post,
    path = "/user/{user_id}/suspend",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = OK, body = User, description = "success")),
)]
async fn user_suspend(
    Path(target_user_id): Path<UserIdReq>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<SuspendRequest>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let d = s.data();
    let srv = s.services();
    let target_user_id = match target_user_id {
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
            expires_at: json.expires_at,
            reason: reason.clone(),
        }),
    )
    .await?;
    let al = auth.audit_log(SERVER_ROOM_ID);
    al.commit_success(AuditLogEntryType::UserSuspend {
        expires_at: json.expires_at,
        user_id: target_user_id,
    })
    .await?;
    srv.users.invalidate(target_user_id).await;
    let user = srv.users.get(target_user_id, Some(auth.user.id)).await?;
    s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;
    Ok(Json(user))
}

/// User unsuspend
#[utoipa::path(
    delete,
    path = "/user/{user_id}/suspend",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = OK, body = User, description = "success")),
)]
async fn user_unsuspend(
    Path(target_user_id): Path<UserIdReq>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let d = s.data();
    let srv = s.services();
    let target_user_id = match target_user_id {
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
#[utoipa::path(
    post,
    path = "/user/{user_id}/presence",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn user_presence_set(
    Path((target_user_id,)): Path<(UserIdReq,)>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<Presence>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;

    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth.user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth.user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    let srv = s.services();
    srv.presence.set_manual(target_user_id, json).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// User list
///
/// Admin only. List all users on this server.
#[utoipa::path(
    get,
    path = "/user",
    tags = ["user", "badge.admin_only"],
    params(
        PaginationQuery<UserId>,
        UserListParams,
    ),
    responses(
        (status = OK, body = PaginationResponse<User>, description = "success"),
    )
)]
async fn user_list(
    Query(paginate): Query<PaginationQuery<UserId>>,
    Query(q): Query<UserListParams>,
    auth: Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    auth.user.ensure_unsuspended()?;
    let srv = s.services();
    let perms = srv.perms.for_server(auth.user.id).await?;
    perms.ensure(Permission::MemberBan)?;

    let data = s.data();
    let mut users = data.user_list(paginate, q.filter).await?;

    for user in &mut users.items {
        user.emails = Some(data.user_email_list(user.id).await?);
    }

    Ok(Json(users))
}

/// Harvest get
#[utoipa::path(
    get,
    path = "/user/@self/harvest",
    tags = ["user"],
    responses(
        (status = OK, body = Harvest, description = "success"),
        (status = NOT_FOUND, description = "no harvest found"),
    )
)]
async fn harvest_get(auth: Auth, State(_s): State<Arc<ServerState>>) -> Result<impl IntoResponse> {
    if auth.user.bot || auth.user.webhook.is_some() || auth.user.puppet.is_some() {
        return Err(Error::BadStatic("bots cannot use this endpoint"));
    }
    auth.user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Harvest create
#[utoipa::path(
    post,
    path = "/user/@self/harvest",
    tags = ["user"],
    responses(
        (status = ACCEPTED, description = "harvest has been queued"),
    )
)]
async fn harvest_create(
    auth: Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<HarvestCreate>,
) -> Result<impl IntoResponse> {
    if auth.user.bot || auth.user.webhook.is_some() || auth.user.puppet.is_some() {
        return Err(Error::BadStatic("bots cannot use this endpoint"));
    }
    auth.user.ensure_unsuspended()?;

    Ok(Error::Unimplemented)
}

/// Harvest download
#[utoipa::path(
    get,
    path = "/internal/harvest/{harvest_id}/{token}/download",
    params(
        ("harvest_id", description = "Harvest ID"),
        ("token", description = "Download token")
    ),
    tags = ["user"],
    responses(
        (status = OK, description = "success"),
    )
)]
async fn harvest_download(
    Path((_harvest_id, _token)): Path<(HarvestId, String)>,
    State(_s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(user_update))
        .routes(routes!(user_get))
        .routes(routes!(user_delete))
        .routes(routes!(user_undelete))
        .routes(routes!(user_audit_logs))
        .routes(routes!(user_room_list))
        .routes(routes!(user_suspend))
        .routes(routes!(user_unsuspend))
        .routes(routes!(guest_create))
        .routes(routes!(user_presence_set))
        .routes(routes!(user_list))
        .routes(routes!(harvest_get))
        .routes(routes!(harvest_create))
        .routes(routes!(harvest_download))
}
