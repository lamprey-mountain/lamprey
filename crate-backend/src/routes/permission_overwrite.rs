use std::sync::Arc;

use axum::{extract::State, Json};
use common::v1::types::{PermissionOverride, PermissionOverrideWithTarget};
use utoipa_axum::{router::OpenApiRouter, routes};

use super::util::Auth;
use crate::error::{Error, Result};
use crate::ServerState;

/// Thread permission override upsert (TODO)
///
/// Upsert a thread permission override
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/permission/{overwrite_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("overwrite_id", description = "Role or user id"),
    ),
    tags = ["thread"],
    responses((status = OK, body = PermissionOverrideWithTarget, description = "success"))
)]
async fn permission_thread_override(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<PermissionOverride>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Thread permission override delete (TODO)
///
/// Delete a thread permission override
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/permission/{overwrite_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("overwrite_id", description = "Role or user id"),
    ),
    tags = ["thread"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn permission_thread_delete(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag permission override upsert (TODO)
///
/// Upsert a tag permission override
#[utoipa::path(
    put,
    path = "/room/{room_id}/tag/{tag_id}/permission/{overwrite_id}",
    params(
        ("room_id", description = "Room id"),
        ("tag_id", description = "Tag id"),
        ("overwrite_id", description = "Role or user id"),
    ),
    tags = ["tag"],
    responses((status = OK, body = PermissionOverrideWithTarget, description = "success"))
)]
async fn permission_tag_override(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<PermissionOverride>,
) -> Result<Json<PermissionOverride>> {
    Err(Error::Unimplemented)
}

/// Tag permission override delete (TODO)
///
/// Delete a tag permission override
#[utoipa::path(
    delete,
    path = "/room/{room_id}/tag/{tag_id}/permission/{overwrite_id}",
    params(
        ("room_id", description = "Room id"),
        ("tag_id", description = "Tag id"),
        ("overwrite_id", description = "Role or user id"),
    ),
    tags = ["tag"],
    responses((status = NO_CONTENT, description = "success"))
)]
async fn permission_tag_delete(
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(permission_thread_override))
        .routes(routes!(permission_thread_delete))
        .routes(routes!(permission_tag_override))
        .routes(routes!(permission_tag_delete))
}
