use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use http::StatusCode;
use types::{
    PaginationQuery, Permission, Role, RoleCreateRequest, RoleId, RolePatch, RoomId, UserId,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::{RoleCreate, RoleDeleteQuery};
use crate::ServerState;

use super::util::Auth;
use crate::error::Result;

/// Role create
#[utoipa::path(
    post,
    path = "/room/{room_id}/role",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["role"],
    responses(
        (status = CREATED, body = Role, description = "success"),
    )
)]
pub async fn role_create(
    Path(room_id): Path<RoomId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(create): Json<RoleCreateRequest>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleManage)?;
    let role = d
        .role_create(RoleCreate {
            room_id,
            name: create.name,
            description: create.description,
            permissions: create.permissions,
            is_self_applicable: create.is_self_applicable,
            is_mentionable: create.is_mentionable,
            is_default: create.is_default,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(role)))
}

/// Role update
#[utoipa::path(
    patch,
    path = "/room/{room_id}/role/{role_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
        (status = NOT_MODIFIED, description = "success"),
    )
)]
pub async fn role_update(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<RolePatch>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleManage)?;
    let role = d.role_select(room_id, role_id).await?;
    if patch.wont_change(&role) {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }
    d.role_update(room_id, role_id, patch).await?;
    let role = d.role_select(room_id, role_id).await?;
    Ok(Json(role).into_response())
}

/// Role delete
#[utoipa::path(
    delete,
    path = "/room/{room_id}/role/{role_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
pub async fn role_delete(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    Query(query): Query<RoleDeleteQuery>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleManage)?;
    let existing = d.role_member_count(role_id).await?;
    if existing == 0 || query.force {
        d.role_delete(room_id, role_id).await?;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::CONFLICT)
    }
}

/// Role get
#[utoipa::path(
    get,
    path = "/room/{room_id}/role/{role_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn role_get(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    let role = d.role_select(room_id, role_id).await?;
    Ok(Json(role))
}

/// Role list
#[utoipa::path(
    get,
    path = "/room/{room_id}/role",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn role_list(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<RoleId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    let res = d.role_list(room_id, paginate).await?;
    Ok(Json(res))
}

/// Role list members
#[utoipa::path(
    get,
    path = "/room/{room_id}/role/{role_id}/member",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn role_member_list(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    let res = d.role_member_list(role_id, paginate).await?;
    Ok(Json(res))
}

/// Role member apply
#[utoipa::path(
    put,
    path = "/room/{room_id}/role/{role_id}/member/{user_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
        ("user_id", description = "User id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn role_member_add(
    Path((room_id, role_id, target_user_id)): Path<(RoomId, RoleId, UserId)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleApply)?;
    d.role_member_put(target_user_id, role_id).await?;
    let member = d.room_member_get(room_id, target_user_id).await?;
    Ok(Json(member))
}

/// Role member remove
#[utoipa::path(
    delete,
    path = "/room/{room_id}/role/{role_id}/member/{user_id}",
    params(
        ("room_id", description = "Room id"),
        ("role_id", description = "Role id"),
        ("user_id", description = "User id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn role_member_remove(
    Path((room_id, role_id, target_user_id)): Path<(RoomId, RoleId, UserId)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleApply)?;
    d.role_member_delete(target_user_id, role_id).await?;
    let member = d.room_member_get(room_id, target_user_id).await?;
    Ok(Json(member))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(role_create))
        .routes(routes!(role_update))
        .routes(routes!(role_delete))
        .routes(routes!(role_get))
        .routes(routes!(role_list))
        .routes(routes!(role_member_list))
        .routes(routes!(role_member_add))
        .routes(routes!(role_member_remove))
}
