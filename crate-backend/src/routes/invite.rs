use axum::{extract::State, Json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use crate::error::Result;
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
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
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
        (status = OK, description = "success"),
    )
)]
pub async fn invite_resolve(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
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
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Invite room create
///
/// Create an invite that goes to a room
#[utoipa::path(
    post,
    path = "/rooms/{room_id}/invite",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn invite_room_create(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Invite room list
///
/// List invites that go to a room
#[utoipa::path(
    get,
    path = "/rooms/{room_id}/invite",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn invite_room_list(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Invite user create
///
/// Create an invite that goes to a user
#[utoipa::path(
    post,
    path = "/users/{user_id}/invite",
    params(
        ("user_id", description = "User id"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn invite_user_create(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Invite user list
///
/// List invites that go to a user
#[utoipa::path(
    get,
    path = "/users/{user_id}/invite",
    params(
        ("user_id", description = "User id"),
    ),
    tags = ["invite"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn invite_user_list(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        // .routes(routes!(invite_delete))
        // .routes(routes!(invite_resolve))
        // .routes(routes!(invite_use))
        // .routes(routes!(invite_room_create))
        // .routes(routes!(invite_user_create))
        // .routes(routes!(invite_user_list))
        // .routes(routes!(invite_room_list))
}
