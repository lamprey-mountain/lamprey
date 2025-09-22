use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::user_status::{Status, StatusPatch};
use common::v1::types::util::{Changes, Diff, Time};
use common::v1::types::{
    application::Connection, ApplicationId, AuditLogEntry, AuditLogEntryId, AuditLogEntryType,
    MediaTrackInfo, MessageSync, PaginationQuery, PaginationResponse, SessionStatus, User,
    UserCreate, UserPatch, UserWithRelationship,
};
use common::v1::types::{Permission, Suspended, SERVER_ROOM_ID};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::routes::util::{AuthWithSession, HeaderReason};
use crate::types::{DbUserCreate, MediaLinkType, UserIdReq};
use crate::ServerState;

use super::util::{Auth, AuthRelaxed};
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
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(patch): Json<UserPatch>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth_user.id != target_user_id {
        return Err(Error::NotFound);
    }
    let data = s.data();
    let srv = s.services();
    let start = srv.users.get(target_user_id).await?;
    if !patch.changes(&start) {
        return Err(Error::NotModified);
    }
    if let Some(Some(avatar_media_id)) = patch.avatar {
        let existing = data.media_link_select(avatar_media_id).await?;
        if !existing.is_empty() {
            return Err(Error::BadStatic("cant reuse media"));
        }

        let (media, _) = data.media_select(avatar_media_id).await?;
        if !matches!(media.source.info, MediaTrackInfo::Image(_)) {
            return Err(Error::BadStatic(
                "couldn't link media as avatar: not an image",
            ));
        }
    }
    data.user_update(target_user_id, patch.clone()).await?;
    data.media_link_delete(target_user_id.into_inner(), MediaLinkType::AvatarUser)
        .await?;
    if let Some(Some(avatar_media_id)) = patch.avatar {
        data.media_link_insert(
            avatar_media_id,
            target_user_id.into_inner(),
            MediaLinkType::AvatarUser,
        )
        .await?;
    }
    srv.users.invalidate(target_user_id).await;
    let user = srv.users.get(target_user_id).await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: target_user_id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::UserUpdate {
            changes: Changes::new()
                .change("name", &start.name, &user.name)
                .change("description", &start.description, &user.description)
                .change("avatar", &start.avatar, &user.avatar)
                .build(),
        },
    })
    .await?;
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
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth_user.id != target_user_id {
        return Err(Error::NotFound);
    }
    let data = s.data();
    data.user_delete(target_user_id).await?;
    data.media_link_delete(target_user_id.into_inner(), MediaLinkType::AvatarUser)
        .await?;
    let srv = s.services();
    srv.users.invalidate(target_user_id).await;
    s.broadcast(MessageSync::UserDelete { id: target_user_id })?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: target_user_id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::UserDelete {
            user_id: target_user_id,
        },
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// User undelete (TODO)
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
    Path(_target_user_id): Path<UserIdReq>,
    Auth(_auth_user): Auth,
    State(_s): State<Arc<ServerState>>,
    HeaderReason(_reason): HeaderReason,
) -> Result<impl IntoResponse> {
    Ok(Error::Unimplemented)
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
    tags = ["user"],
    responses(
        (status = OK, body = UserWithRelationship, description = "success"),
    )
)]
async fn user_get(
    Path(target_user_id): Path<UserIdReq>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let user = srv.users.get(target_user_id).await?;
    let data = s.data();
    let relationship = data
        .user_relationship_get(auth_user.id, target_user_id)
        .await?
        .unwrap_or_default();
    Ok(Json(UserWithRelationship {
        inner: user,
        relationship,
    }))
}

/// User audit logs (TODO)
#[utoipa::path(
    get,
    path = "/user/{user_id}/audit-logs",
    params(
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };

    if auth_user.id != target_user_id {
        return Err(Error::NotFound);
    }

    let data = s.data();
    let logs = data
        .audit_logs_room_fetch(target_user_id.into_inner().into(), paginate)
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
            bot: None,
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

    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: user.id.into_inner().into(),
        user_id: user.id,
        session_id: Some(updated_session.id),
        reason: None,
        ty: AuditLogEntryType::SessionLogin {
            user_id: user.id,
            session_id: updated_session.id,
        },
    })
    .await?;

    Ok((StatusCode::CREATED, Json(user)))
}

#[derive(Deserialize, ToSchema)]
struct SuspendRequest {
    expires_at: Option<Time>,
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<SuspendRequest>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let d = s.data();
    let srv = s.services();
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if target_user_id != auth_user.id {
        let perms = srv.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;
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
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user.id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::UserSuspend {
            expires_at: json.expires_at,
            user_id: target_user_id,
        },
    })
    .await?;
    srv.users.invalidate(target_user_id).await;
    let user = srv.users.get(target_user_id).await?;
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
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;
    let d = s.data();
    let srv = s.services();
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let perms = srv.perms.for_room(auth_user.id, SERVER_ROOM_ID).await?;
    perms.ensure(Permission::MemberBan)?;
    d.user_suspended(target_user_id, None).await?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: SERVER_ROOM_ID,
        user_id: auth_user.id,
        session_id: None,
        reason,
        ty: AuditLogEntryType::UserUnsuspend {
            user_id: target_user_id,
        },
    })
    .await?;
    srv.users.invalidate(target_user_id).await;
    let user = srv.users.get(target_user_id).await?;
    s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;
    Ok(Json(user))
}

/// Connection list
#[utoipa::path(
    get,
    path = "/user/{user_id}/connection",
    params(
        ("user_id", description = "User id"),
        PaginationQuery<ApplicationId>
    ),
    tags = ["user"],
    responses((status = OK, body = PaginationResponse<Connection>, description = "success")),
)]
async fn connection_list(
    Path(target_user_id): Path<UserIdReq>,
    Query(paginate): Query<PaginationQuery<ApplicationId>>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth_user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    let connections = s.data().connection_list(target_user_id, paginate).await?;
    Ok(Json(connections))
}

/// Connection revoke
#[utoipa::path(
    delete,
    path = "/user/{user_id}/connection/{app_id}",
    params(
        ("user_id", description = "User id"),
        ("app_id", description = "Application id")
    ),
    tags = ["user"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn connection_revoke(
    Path((target_user_id, app_id)): Path<(UserIdReq, ApplicationId)>,
    AuthWithSession(session, auth_user): AuthWithSession,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth_user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    s.data().connection_delete(target_user_id, app_id).await?;

    s.broadcast(MessageSync::ConnectionDelete {
        user_id: target_user_id,
        app_id,
    })?;
    s.audit_log_append(AuditLogEntry {
        id: AuditLogEntryId::new(),
        room_id: target_user_id.into_inner().into(),
        user_id: auth_user.id,
        session_id: Some(session.id),
        reason,
        ty: AuditLogEntryType::ConnectionDelete {
            application_id: app_id,
        },
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// User set status
///
/// for puppets
#[utoipa::path(
    post,
    path = "/user/{user_id}/status",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = NO_CONTENT, description = "success")),
)]
async fn user_set_status(
    Path((target_user_id,)): Path<(UserIdReq,)>,
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<StatusPatch>,
) -> Result<impl IntoResponse> {
    auth_user.ensure_unsuspended()?;

    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user.id,
        UserIdReq::UserId(id) => id,
    };

    if auth_user.id != target_user_id {
        return Err(Error::MissingPermissions);
    }

    let srv = s.services();
    srv.users
        .status_set(target_user_id, json.apply(Status::offline()))
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(user_update))
        .routes(routes!(user_get))
        .routes(routes!(user_delete))
        .routes(routes!(user_undelete))
        .routes(routes!(user_audit_logs))
        .routes(routes!(user_suspend))
        .routes(routes!(user_unsuspend))
        .routes(routes!(connection_list))
        .routes(routes!(connection_revoke))
        .routes(routes!(guest_create))
        .routes(routes!(user_set_status))
}
