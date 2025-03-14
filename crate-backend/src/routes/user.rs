use std::sync::Arc;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::Diff;
use common::v1::types::{
    BotOwner, MediaTrackInfo, MessageSync, PaginationQuery, PaginationResponse, User, UserCreate,
    UserId, UserPatch, UserType,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::{DbUserCreate, MediaLinkType, UserIdReq};
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// User create
#[utoipa::path(
    post,
    path = "/user",
    tags = ["user"],
    responses(
        (status = CREATED, body = User, description = "success"),
    )
)]
async fn user_create(
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(body): Json<UserCreate>,
) -> Result<impl IntoResponse> {
    let parent_id = Some(auth_user_id);
    let data = s.data();
    let srv = s.services();
    let parent = srv.users.get(auth_user_id).await?;
    if !parent.user_type.can_create(&body.user_type) {
        return Err(Error::BadStatic("can't create that user"));
    };
    match &body.user_type {
        UserType::Bot { owner, .. } => match owner {
            BotOwner::User { user_id } if *user_id != auth_user_id => {
                return Err(Error::BadStatic("bad owner id"));
            }
            _ => {}
        },
        UserType::Puppet {
            owner_id, alias_id, ..
        } => {
            if alias_id.is_some() {
                return Err(Error::Unimplemented);
            }
            if *owner_id != auth_user_id {
                return Err(Error::BadStatic("bad owner id"));
            }
        }
        _ => {}
    };
    let user = data
        .user_create(DbUserCreate {
            parent_id,
            name: body.name,
            description: body.description,
            user_type: body.user_type,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(user)))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum UserListFilter {
    /// users in mutual rooms, excluding puppets
    Mutual,

    /// friends
    Friends,

    /// users you have dms with
    Dms,

    /// puppets in mutual rooms
    Puppet,

    /// to bots you have created
    Bot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, IntoParams)]
struct UserListParams {
    #[serde(default = "default_user_list_filter")]
    include: Vec<UserListFilter>,
}

fn default_user_list_filter() -> Vec<UserListFilter> {
    vec![
        UserListFilter::Mutual,
        UserListFilter::Friends,
        UserListFilter::Dms,
    ]
}

/// User list (TODO)
///
/// Lists every user you are able to see. Can be filtered with ?include
#[utoipa::path(
    get,
    path = "/user",
    tags = ["user"],
    params(
        UserListParams,
        PaginationQuery<UserId>,
    ),
    responses(
        (status = OK, body = PaginationResponse<User>, description = "success"),
    )
)]
async fn user_list(Auth(_session): Auth, State(_s): State<Arc<ServerState>>) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// User update
// TODO: updating/deleting bots
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
    s.broadcast(MessageSync::UpsertUser { user: user.clone() })?;
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
    s.broadcast(MessageSync::DeleteUser { id: target_user_id })?;
    Ok(StatusCode::NO_CONTENT)
}

/// User get
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
    Ok(Json(user))
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

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(user_create))
        .routes(routes!(user_list))
        .routes(routes!(user_update))
        .routes(routes!(user_get))
        .routes(routes!(user_delete))
        .routes(routes!(user_audit_logs))
}
