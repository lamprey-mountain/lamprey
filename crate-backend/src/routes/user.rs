use std::sync::Arc;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use types::{MessageSync, UserCreateRequest, UserPatch};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::{UserCreate, UserIdReq};
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// User create
#[utoipa::path(
    post,
    path = "/user",
    tags = ["user"],
    responses(
        (status = CREATED, description = "success"),
    )
)]
pub async fn user_create(
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
    Json(body): Json<UserCreateRequest>,
) -> Result<impl IntoResponse> {
    let parent_id = Some(session.user_id);
    let data = s.data();
    let user = data
        .user_create(UserCreate {
            parent_id,
            name: body.name,
            description: body.description,
            status: body.status,
            is_bot: body.is_bot,
            is_alias: body.is_alias,
            is_system: false,
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
//     Auth(session): Auth,
// State(s): State<Arc<ServerState>>,
// ) -> Result<Json<()>> {
//     todo!()
// }

/// User update
#[utoipa::path(
    patch,
    path = "/user/{user_id}",
    params(
        ("user_id", description = "User id"),
    ),
    tags = ["user"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn user_update(
    Path(user_id): Path<UserIdReq>,
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
    Json(body): Json<UserPatch>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id {
        UserIdReq::UserSelf => session.user_id,
        UserIdReq::UserId(user_id) => user_id,
    };
    if session.user_id != user_id {
        return Err(Error::MissingPermissions);
    }
    let data = s.data();
    data.user_update(user_id, body).await?;
    let user = data.user_get(user_id).await?;
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
    Path(user_id): Path<UserIdReq>,
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id {
        UserIdReq::UserSelf => session.user_id,
        UserIdReq::UserId(user_id) => user_id,
    };
    if session.user_id != user_id {
        return Err(Error::MissingPermissions);
    }
    let data = s.data();
    data.user_delete(user_id).await?;
    s.broadcast(MessageSync::DeleteUser { id: user_id })?;
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
        (status = OK, description = "success"),
    )
)]
pub async fn user_get(
    Path(user_id): Path<UserIdReq>,
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let user_id = match user_id {
        UserIdReq::UserSelf => session.user_id,
        UserIdReq::UserId(user_id) => user_id,
    };
    let data = s.data();
    let user = data.user_get(user_id).await?;
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
