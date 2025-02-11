use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use http::StatusCode;
use types::{
    MessageSync, PaginationQuery, PaginationResponse, Permission, RoomId, RoomMember,
    RoomMemberPatch, RoomMembership, UserId,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::Auth;
use crate::error::{Error, Result};

/// Room member list
#[utoipa::path(
    get,
    path = "/room/{room_id}/member",
    params(
        PaginationQuery<UserId>,
        ("room_id" = RoomId, description = "Room id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = PaginationResponse<RoomMember>, description = "success"),
    )
)]
pub async fn room_member_list(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    let res = d.room_member_list(room_id, paginate).await?;
    Ok(Json(res))
}

/// Room member get
#[utoipa::path(
    get,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
    )
)]
pub async fn room_member_get(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = d.permission_room_get(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    let res = d.room_member_get(room_id, target_user_id).await?;
    // TODO: return `Ban`s
    if !matches!(res.membership, RoomMembership::Join { .. }) {
        Err(Error::NotFound)
    } else {
        Ok(Json(res))
    }
}

/// Room member update
#[utoipa::path(
    patch,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
pub async fn room_member_update(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(patch): Json<RoomMemberPatch>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = d.permission_room_get(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    if target_user_id != auth_user_id {
        perms.ensure(Permission::MemberManage)?;
    }

    let start = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(start.membership, RoomMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    d.room_member_patch(room_id, target_user_id, patch).await?;
    let res = d.room_member_get(room_id, target_user_id).await?;
    if start == res {
        Ok(StatusCode::NOT_MODIFIED.into_response())
    } else {
        s.broadcast_room(
            room_id,
            auth_user_id,
            None,
            MessageSync::UpsertRoomMember {
                member: res.clone(),
            },
        )
        .await?;
        Ok(Json(res).into_response())
    }
}

/// Room member delete (kick/leave)
#[utoipa::path(
    delete,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["room_member"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
pub async fn room_member_delete(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = d.permission_room_get(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    if target_user_id != auth_user_id {
        perms.ensure(Permission::MemberKick)?;
    }
    let start = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(start.membership, RoomMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    d.room_member_set_membership(room_id, target_user_id, RoomMembership::Leave {})
        .await?;
    s.services().perms.invalidate_room(target_user_id, room_id);
    let res = d.room_member_get(room_id, target_user_id).await?;
    s.broadcast_room(
        room_id,
        auth_user_id,
        None,
        MessageSync::UpsertRoomMember { member: res },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(room_member_list))
        .routes(routes!(room_member_get))
        .routes(routes!(room_member_update))
        .routes(routes!(room_member_delete))
}
