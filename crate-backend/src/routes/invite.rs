use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use http::StatusCode;
use nanoid::nanoid;
use serde::Serialize;
use types::{
    Invite, InviteCode, InviteTarget, InviteTargetId, InviteWithMetadata, MessageSync,
    PaginationQuery, PaginationResponse, Permission, RoomId, RoomMemberPut, RoomMembership,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::ServerState;

use super::util::Auth;

/// Invite delete
#[utoipa::path(
    delete,
    path = "/invite/{invite_code}",
    params(
        ("invite_code", description = "The code identifying this invite"),
    ),
    tags = ["invite"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
pub async fn invite_delete(
    Path(code): Path<InviteCode>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let invite = d.invite_select(code.clone()).await?;
    let (has_perm, id_target) = match invite.invite.target {
        InviteTarget::User { user } => (
            user.id == user_id,
            InviteTargetId::User { user_id: user.id },
        ),
        InviteTarget::Room { room } => (
            d.permission_room_get(user_id, room.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Room { room_id: room.id },
        ),
        InviteTarget::Thread { room, thread } => (
            d.permission_thread_get(user_id, thread.id)
                .await?
                .has(Permission::InviteManage),
            InviteTargetId::Thread {
                room_id: room.id,
                thread_id: thread.id,
            },
        ),
    };
    let can_delete = user_id == invite.invite.creator.id || has_perm;
    if can_delete {
        d.invite_delete(code.clone()).await?;
        s.broadcast(MessageSync::DeleteInvite {
            code,
            target: id_target,
        })?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Invite resolve
#[utoipa::path(
    get,
    path = "/invite/{invite_code}",
    params(
        ("invite_code", description = "The code identifying this invite"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, body = Invite, description = "success"),
        (status = OK, body = InviteWithMetadata, description = "success with metadata"),
    )
)]
pub async fn invite_resolve(
    Path(code): Path<InviteCode>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let invite = d.invite_select(code).await?;
    if invite.invite.creator.id == user_id {
        return Ok(Json(invite).into_response());
    }
    let should_strip = match &invite.invite.target {
        InviteTarget::User { user } => user.id != user_id,
        InviteTarget::Room { room } => {
            let perms = d.permission_room_get(user_id, room.id).await?;
            !perms.has(Permission::InviteManage)
        }
        InviteTarget::Thread { room: _, thread } => {
            let perms = d.permission_thread_get(user_id, thread.id).await?;
            !perms.has(Permission::InviteManage)
        }
    };
    if should_strip {
        Ok(Json(invite.strip_metadata()).into_response())
    } else {
        Ok(Json(invite).into_response())
    }
}

/// Invite use
#[utoipa::path(
    post,
    path = "/invite/{invite_code}",
    params(
        ("invite_code", description = "The code identifying this invite"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn invite_use(
    Path(code): Path<InviteCode>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let invite = d.invite_select(code).await?;
    match invite.invite.target {
        InviteTarget::User { user: _ } => todo!("dms aren't implemented"),
        InviteTarget::Thread { room, .. } | InviteTarget::Room { room } => {
            // TODO: any thread-specific invite thing?
            d.room_member_put(RoomMemberPut {
                user_id,
                room_id: room.id,
                membership: RoomMembership::Join,
                override_name: None,
                override_description: None,
                roles: vec![],
            })
            .await?;
            d.role_apply_default(room.id, user_id).await?;
            let member = d.room_member_get(room.id, user_id).await?;
            s.broadcast(MessageSync::UpsertRoomMember { member })?;
        }
    }
    Ok(())
}

/// Invite room create
///
/// Create an invite that goes to a room
#[utoipa::path(
    post,
    path = "/room/{room_id}/invite",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, body = Invite, description = "success"),
    )
)]
pub async fn invite_room_create(
    Path(room_id): Path<RoomId>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::InviteCreate)?;
    let alphabet: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let code = InviteCode(nanoid!(8, &alphabet));
    d.invite_insert_room(room_id, user_id, code.clone()).await?;
    let invite = d.invite_select(code).await?;
    s.broadcast(MessageSync::UpsertInvite {
        invite: invite.clone(),
    })?;
    Ok((StatusCode::CREATED, Json(invite)))
}

#[derive(Serialize)]
#[serde(untagged)]
enum InviteWithPotentialMetadata {
    Invite(Invite),
    InviteWithMetadata(InviteWithMetadata),
}

/// Invite room list
///
/// List invites that go to a room
#[utoipa::path(
    get,
    path = "/room/{room_id}/invite",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, body = PaginationResponse<Invite>, description = "success"),
    )
)]
pub async fn invite_room_list(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<InviteCode>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = d.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    let res = d.invite_list_room(room_id, paginate).await?;
    let res = PaginationResponse {
        items: res
            .items
            .into_iter()
            .map(|i| {
                if i.invite.creator.id != user_id && !perms.has(Permission::InviteManage) {
                    InviteWithPotentialMetadata::Invite(i.strip_metadata())
                } else {
                    InviteWithPotentialMetadata::InviteWithMetadata(i)
                }
            })
            .collect(),
        total: res.total,
        has_more: res.has_more,
    };
    Ok(Json(res))
}

// /// Invite user create
// ///
// /// Create an invite that goes to a user
// #[utoipa::path(
//     post,
//     path = "/users/{user_id}/invite",
//     params(
//         ("user_id", description = "User id"),
//     ),
//     tags = ["invite"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
// pub async fn invite_user_create(
//     Auth(user_id): Auth,
//     State(s): State<Arc<ServerState>>,
// ) -> Result<impl IntoResponse> {
//     Ok(StatusCode::NOT_IMPLEMENTED)
// }

// /// Invite user list
// ///
// /// List invites that go to a user
// #[utoipa::path(
//     get,
//     path = "/users/{user_id}/invite",
//     params(
//         ("user_id", description = "User id"),
//     ),
//     tags = ["invite"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
// pub async fn invite_user_list(
//     Auth(user_id): Auth,
//     State(s): State<Arc<ServerState>>,
// ) -> Result<impl IntoResponse> {
//     Ok(StatusCode::NOT_IMPLEMENTED)
// }

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(invite_delete))
        .routes(routes!(invite_resolve))
        .routes(routes!(invite_use))
        .routes(routes!(invite_room_create))
        .routes(routes!(invite_room_list))
    // .routes(routes!(invite_user_create))
    // .routes(routes!(invite_user_list))
}
