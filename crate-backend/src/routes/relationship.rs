use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::{extract::State, Json};
use types::{PaginationQuery, PaginationResponse, Relationship, RelationshipPatch, UserId};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// Relationship get (TODO)
///
/// Get your relationship with another user
#[utoipa::path(
    get,
    path = "/user/@self/relationship/{target_id}",
    params(
        ("target_id", description = "Target user's id"),
    ),
    tags = ["relationship"],
    responses(
        (status = OK, body = Relationship, description = "success"),
    )
)]
async fn relationship_get(
    Path(_target_user_id): Path<UserId>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Relationship update (TODO)
///
/// Update your relationship with another user
#[utoipa::path(
    patch,
    path = "/user/@self/relationship/{target_id}",
    params(
        ("target_id", description = "Target user's id"),
    ),
    tags = ["relationship"],
    responses(
        (status = OK, body = Relationship, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn relationship_update(
    Path(_target_user_id): Path<UserId>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_patch): Json<RelationshipPatch>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Relationship remove (TODO)
///
/// Reset your relationship with another user
#[utoipa::path(
    delete,
    path = "/user/@self/relationship/{target_id}",
    params(
        ("target_id", description = "Target user's id"),
    ),
    tags = ["relationship"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn relationship_reset(
    Path(_target_user_id): Path<UserId>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Relationship list (TODO)
///
/// List relationships with other users. Passing in someone else's id lists
/// mutual friends.
#[utoipa::path(
    get,
    path = "/user/{user_id}/relationship",
    params(
        PaginationQuery<UserId>,
        ("user_id", description = "User id to list relationships from"),
    ),
    tags = ["relationship"],
    responses(
        (status = OK, body = PaginationResponse<Relationship>, description = "success"),
    )
)]
async fn relationship_list(
    Path(_target_user_id): Path<UserIdReq>,
    Auth(_auth_user_id): Auth,
    Query(_q): Query<PaginationQuery<UserId>>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(relationship_get))
        .routes(routes!(relationship_update))
        .routes(routes!(relationship_reset))
        .routes(routes!(relationship_list))
}
