use std::sync::Arc;

use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use common::v1::types::email::EmailAddr;
use common::v1::types::email::EmailInfo;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::UserIdReq;
use crate::ServerState;

use crate::error::{Error, Result};

use super::util::Auth;

/// Email add
#[utoipa::path(
    put,
    path = "/user/{user_id}/email/{addr}",
    params(
        ("user_id", description = "User id"),
        ("addr", description = "email address"),
    ),
    tags = ["user_email"],
    responses(
        (status = CREATED, description = "success"),
        (status = OK, description = "already exists"),
    ),
)]
async fn email_add(
    Path((_target_user_id, _email)): Path<(UserIdReq, EmailAddr)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Email get
#[utoipa::path(
    get,
    path = "/user/{user_id}/email/{addr}",
    params(
        ("user_id", description = "User id"),
        ("addr", description = "email address"),
    ),
    tags = ["user_email"],
    responses(
        (status = OK, body = EmailInfo, description = "success"),
        (status = NOT_FOUND, description = "doesn't exist"),
    ),
)]
async fn email_get(
    Path((_target_user_id, _email)): Path<(UserIdReq, EmailAddr)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Email delete
#[utoipa::path(
    delete,
    path = "/user/{user_id}/email/{addr}",
    params(
        ("user_id", description = "User id"),
        ("addr", description = "email address"),
    ),
    tags = ["user_email"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn email_delete(
    Path((_target_user_id, _email)): Path<(UserIdReq, EmailAddr)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Email list
#[utoipa::path(
    get,
    path = "/user/{user_id}/email",
    params(("user_id", description = "User id")),
    tags = ["user_email"],
    responses((status = OK, body = Vec<EmailInfo>, description = "success"))
)]
async fn email_list(
    Path((_target_user_id, _email)): Path<(UserIdReq, EmailAddr)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(email_add))
        .routes(routes!(email_get))
        .routes(routes!(email_list))
        .routes(routes!(email_delete))
}
