use std::sync::Arc;

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use types::{Invite, InviteCode, InviteTarget, InviteWithMetadata, Permission};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use crate::error::Result;
use super::util::Auth;

// /// Invite delete
// #[utoipa::path(
//     delete,
//     path = "/invite/{invite_code}",
//     params(
//         ("invite_code", description = "The code identifying this invite"),
//     ),
//     tags = ["invite"],
//     responses(
//         (status = NO_CONTENT, description = "success"),
//     )
// )]
// pub async fn invite_delete(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

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
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let invite = d.invite_select(code).await?;
    if invite.invite.creator.id == session.user_id {
        return Ok(Json(invite).into_response())
    }
    let should_strip = match &invite.invite.target {
        InviteTarget::User { user } => user.id == session.user_id,
        InviteTarget::Room { room } => {
            let perms = d.permission_room_get(session.user_id, room.id).await?;
            perms.has(Permission::InviteManage)
        },
        InviteTarget::Thread { room: _, thread } => {
            let perms = d.permission_thread_get(session.user_id, thread.id).await?;
            perms.has(Permission::InviteManage)
        },
    };
    if should_strip {
        Ok(Json(invite.strip_metadata()).into_response())
    } else {
        Ok(Json(invite).into_response())
    }
}

// /// Invite use
// #[utoipa::path(
//     post,
//     path = "/invite/{invite_code}",
//     params(
//         ("invite_code", description = "The code identifying this invite"),
//     ),
//     tags = ["invite"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
// pub async fn invite_use(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

// /// Invite room create
// ///
// /// Create an invite that goes to a room
// #[utoipa::path(
//     post,
//     path = "/rooms/{room_id}/invite",
//     params(
//         ("room_id", description = "Room id"),
//     ),
//     tags = ["invite"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
// pub async fn invite_room_create(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

// /// Invite room list
// ///
// /// List invites that go to a room
// #[utoipa::path(
//     get,
//     path = "/rooms/{room_id}/invite",
//     params(
//         ("room_id", description = "Room id"),
//     ),
//     tags = ["invite"],
//     responses(
//         (status = OK, description = "success"),
//     )
// )]
// pub async fn invite_room_list(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

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
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
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
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        // .routes(routes!(invite_delete))
        .routes(routes!(invite_resolve))
        // .routes(routes!(invite_use))
        // .routes(routes!(invite_room_create))
        // .routes(routes!(invite_user_create))
        // .routes(routes!(invite_user_list))
        // .routes(routes!(invite_room_list))
}
