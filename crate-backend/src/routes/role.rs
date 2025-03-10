use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::Diff;
use common::v1::types::{
    MessageSync, PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId,
    RolePatch, RoomId, RoomMember, RoomMembership, UserId,
};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::types::{DbRoleCreate, RoleDeleteQuery};
use crate::ServerState;

use super::util::{Auth, HeaderReason};
use crate::error::{Error, Result};

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
    HeaderReason(reason): HeaderReason,
    Json(json): Json<RoleCreate>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let d = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleManage)?;
    let role = d
        .role_create(DbRoleCreate {
            room_id,
            name: json.name,
            description: json.description,
            permissions: json.permissions,
            is_self_applicable: json.is_self_applicable,
            is_mentionable: json.is_mentionable,
            is_default: json.is_default,
        })
        .await?;
    let msg = MessageSync::UpsertRole { role: role.clone() };
    s.broadcast_room(room_id, user_id, reason, msg).await?;
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
        (status = OK, body = Role, description = "success"),
        (status = NOT_MODIFIED, description = "success"),
    )
)]
pub async fn role_update(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<RolePatch>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let d = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleManage)?;
    let role = d.role_select(room_id, role_id).await?;
    if !json.changes(&role) {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }
    d.role_update(room_id, role_id, json.clone()).await?;
    let role = d.role_select(room_id, role_id).await?;
    let msg = MessageSync::UpsertRole { role: role.clone() };
    if json.permissions.is_some_and(|p| p != role.permissions) {
        s.services().perms.invalidate_room_all(room_id);
    }
    s.broadcast_room(room_id, user_id, reason, msg).await?;
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
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleManage)?;
    let existing = d.role_member_count(role_id).await?;
    if existing == 0 || query.force {
        d.role_delete(room_id, role_id).await?;
        let msg = MessageSync::DeleteRole { room_id, role_id };
        s.services().perms.invalidate_room_all(room_id);
        s.broadcast_room(room_id, user_id, reason, msg).await?;
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
        (status = OK, body = Role, description = "success"),
    )
)]
pub async fn role_get(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    let role = d.role_select(room_id, role_id).await?;
    Ok(Json(role))
}

/// Role list
#[utoipa::path(
    get,
    path = "/room/{room_id}/role",
    params(
        PaginationQuery<RoleId>,
        ("room_id", description = "Room id"),
    ),
    tags = ["role"],
    responses(
        (status = OK, body = PaginationResponse<Role>, description = "success"),
    )
)]
pub async fn role_list(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<RoleId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
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
        (status = OK, body = PaginationResponse<RoomMember>, description = "success"),
    )
)]
pub async fn role_member_list(
    Path((room_id, role_id)): Path<(RoomId, RoleId)>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
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
        (status = OK, body = RoomMember, description = "success"),
    )
)]
pub async fn role_member_add(
    Path((room_id, role_id, target_user_id)): Path<(RoomId, RoleId, UserId)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleApply)?;
    d.role_member_put(target_user_id, role_id).await?;
    let member = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(member.membership, RoomMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    let msg = MessageSync::UpsertRoomMember {
        member: member.clone(),
    };
    s.services()
        .perms
        .invalidate_room(target_user_id, room_id)
        .await;
    s.broadcast_room(room_id, auth_user_id, reason, msg).await?;
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
        (status = OK, body = RoomMember, description = "success"),
    )
)]
pub async fn role_member_remove(
    Path((room_id, role_id, target_user_id)): Path<(RoomId, RoleId, UserId)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoleApply)?;
    d.role_member_delete(target_user_id, role_id).await?;
    let member = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(member.membership, RoomMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    let msg = MessageSync::UpsertRoomMember {
        member: member.clone(),
    };
    s.services()
        .perms
        .invalidate_room(target_user_id, room_id)
        .await;
    s.broadcast_room(room_id, auth_user_id, reason, msg).await?;
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
