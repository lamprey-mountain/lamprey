use std::sync::Arc;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::{Diff, Time};
use common::v1::types::{
    MediaTrackInfo, MessageSync, SessionStatus, User, UserCreate, UserPatch, UserWithRelationship,
};
use utoipa_axum::{router::OpenApiRouter, routes};

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
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<UserPatch>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth_user_id != target_user_id {
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
    s.broadcast(MessageSync::UserUpdate { user: user.clone() })?;
    Ok(Json(user))
}

/// User delete
#[utoipa::path(
    delete,
    path = "/user/{user_id}",
    params(
        ("user_id", description = "User id"),
    ),
    tags = ["user"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn user_delete(
    Path(target_user_id): Path<UserIdReq>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    if auth_user_id != target_user_id {
        return Err(Error::NotFound);
    }
    let data = s.data();
    data.user_delete(target_user_id).await?;
    data.media_link_delete(target_user_id.into_inner(), MediaLinkType::AvatarUser)
        .await?;
    let srv = s.services();
    srv.users.invalidate(target_user_id).await;
    s.broadcast(MessageSync::UserDelete { id: target_user_id })?;
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
    tags = ["user"],
    responses(
        (status = OK, body = User, description = "success"),
    )
)]
async fn user_get(
    Path(target_user_id): Path<UserIdReq>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let srv = s.services();
    let user = srv.users.get(target_user_id).await?;
    let data = s.data();
    let relationship = data
        .user_relationship_get(auth_user_id, target_user_id)
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
        (status = OK, body = User, description = "success"),
    )
)]
async fn user_audit_logs(
    Path(_target_user_id): Path<UserIdReq>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<()> {
    Err(Error::Unimplemented)
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

    // Create a new guest user
    let user = data
        .user_create(DbUserCreate {
            parent_id: None,
            name: create.name,
            description: create.description,
            bot: None, // No longer using bot for guest status
            puppet: None,
            registered_at: None, // Mark as guest
        })
        .await?;

    // Associate the current session with the new guest user
    data.session_set_status(session.id, SessionStatus::Authorized { user_id: user.id })
        .await?;
    srv.sessions.invalidate(session.id).await; // Invalidate old session to force reload
    let updated_session = srv.sessions.get(session.id).await?; // Get the updated session
    s.broadcast(MessageSync::SessionCreate {
        session: updated_session,
    })?; // Broadcast session update

    Ok((StatusCode::CREATED, Json(user)))
}

/// User suspend (TODO)
#[utoipa::path(
    post,
    path = "/user/{user_id}/suspend",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = OK, body = User, description = "success")),
)]
async fn user_suspend(
    Path(_target_user_id): Path<UserIdReq>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<()> {
    Err(Error::Unimplemented)
}

/// User unsuspend (TODO)
#[utoipa::path(
    delete,
    path = "/user/{user_id}/suspend",
    params(("user_id", description = "User id")),
    tags = ["user"],
    responses((status = OK, body = User, description = "success")),
)]
async fn user_unsuspend(
    Path(_target_user_id): Path<UserIdReq>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<()> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(user_update))
        .routes(routes!(user_get))
        .routes(routes!(user_delete))
        .routes(routes!(user_audit_logs))
        .routes(routes!(user_suspend))
        .routes(routes!(user_unsuspend))
        .routes(routes!(guest_create))
}
