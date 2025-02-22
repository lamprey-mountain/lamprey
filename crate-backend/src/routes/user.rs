use std::sync::Arc;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use types::util::Diff;
use types::{MediaTrackInfo, MessageSync, User, UserCreate, UserPatch};
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
pub async fn user_create(
    // NOTE: utoipa + cargo check seems to break with _session here?
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(body): Json<UserCreate>,
) -> Result<impl IntoResponse> {
    let parent_id = Some(auth_user_id);
    let data = s.data();
    let user = data
        .user_create(DbUserCreate {
            parent_id,
            name: body.name,
            description: body.description,
            status: body.status,
            is_bot: body.is_bot,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(user)))
}

// TODO: not sure how to implement this
// /// User list
// #[utoipa::path(
//     get,
//     path = "/user",
//     tags = ["user"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
// pub async fn user_list(
//     Auth(_session): Auth,
// State(s): State<Arc<ServerState>>,
// ) -> Result<Json<()>> {
//     todo!()
// }

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
pub async fn user_update(
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
    let start = data.user_get(target_user_id).await?;
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
    let user = data.user_get(target_user_id).await?;
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
pub async fn user_delete(
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
pub async fn user_get(
    Path(target_user_id): Path<UserIdReq>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(target_user_id) => target_user_id,
    };
    let data = s.data();
    let user = data.user_get(target_user_id).await?;
    Ok(Json(user))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(user_create))
        // .routes(routes!(user_list))
        .routes(routes!(user_update))
        .routes(routes!(user_get))
        .routes(routes!(user_delete))
}
