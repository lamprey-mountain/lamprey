use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::TypedHeader;
use headers::ETag;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    error::Result,
    types::{
        MessageSync, PaginationQuery, PaginationResponse, Permission, Room, RoomCreate, RoomId,
        RoomPatch,
    },
    ServerState,
};

use super::util::Auth;

/// Create a room
#[utoipa::path(
    post,
    path = "/room",
    tags = ["room"],
)]
#[axum::debug_handler]
async fn room_create(
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RoomCreate>,
) -> Result<impl IntoResponse> {
    let room = s.services().create_room(json, user_id).await?;
    s.broadcast(MessageSync::UpsertRoom { room: room.clone() })?;
    Ok((StatusCode::CREATED, Json(room)))
}

/// Get a room by its id.
#[utoipa::path(
    get,
    path = "/room/{room_id}",
    tags = ["room"],
    params(("room_id", description = "Room id")),
    responses(
        (status = OK, description = "Get room success", body = Room),
        (status = NOT_MODIFIED, description = "Not modified"),
    )
)]
async fn room_get(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(user_id): Auth,
    headers: HeaderMap,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = data.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    let room = data.room_get(room_id).await?;

    // TODO: use typedheader once the empty if-none-match bug is fixed
    // TODO: last-modified
    let etag = format!(r#"W/"{}""#, room.version_id);

    if let Some(if_none_match) = headers.get("if-none-match") {
        if if_none_match == &etag {
            return Ok(StatusCode::NOT_MODIFIED.into_response());
        }
    }

    let etag: ETag = etag.parse().unwrap();
    Ok((TypedHeader(etag), Json(room)).into_response())
}

/// List visible rooms
#[utoipa::path(
    get,
    path = "/room",
    tags = ["room"],
    params(PaginationQuery<RoomId>),
    responses(
        (status = 200, description = "Paginate room success", body = PaginationResponse<Room>),
    )
)]
async fn room_list(
    Query(q): Query<PaginationQuery<RoomId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let res = data.room_list(user_id, q).await?;
    Ok(Json(res))
}

/// edit a room
#[utoipa::path(
    patch,
    path = "/room/{room_id}",
    params(
        ("room_id", description = "Room id"),
    ),
    tags = ["room"],
    responses(
        (status = OK, description = "edit success"),
        (status = NOT_MODIFIED, description = "no change"),
    )
)]
async fn room_edit(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<RoomPatch>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let perms = data.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::RoomManage)?;
    data.room_update(room_id, json).await?;
    let room = data.room_get(room_id).await?;
    s.broadcast(MessageSync::UpsertRoom { room: room.clone() })?;
    Ok(Json(room))
}

// /// ack message
// ///
// /// Mark all threads in a room as read.
// #[utoipa::path(
//     put,
//     path = "/room/{room_id}/ack",
//     params(
//         ("room_id", description = "Room id"),
//     ),
//     tags = ["room"],
//     responses(
//         (status = NO_CONTENT, description = "success"),
//     )
// )]
// async fn room_ack(
//     Path((room_id,)): Path<(RoomId,)>,
//     Auth(user_id): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

// /// dm initialize
// /// Get or create a direct message room.
// #[utoipa::path(
//     patch,
//     path = "/dm/{user_id}",
//     params(
//         ("user_id", description = "Target user's id"),
//     ),
//     tags = ["room"],
//     responses(
//         (status = CREATED, description = "new dm created"),
//         (status = OK, description = "already exists"),
//     )
// )]
// async fn dm_initialize(
//     Path((user_id, )): Path<(UserId,)>,
//     Auth(user_id): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<Room>> {
//     todo!()
// }

// /// dm get
// /// Get a direct message room.
// #[utoipa::path(
//     get,
//     path = "/dm/{user_id}",
//     params(
//         ("user_id", description = "Target user's id"),
//     ),
//     tags = ["room"],
//     responses(
//         (status = OK, description = "already exists"),
//     )
// )]
// async fn dm_get(
//     Path((user_id, )): Path<(UserId,)>,
//     Auth(user_id): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<Room>> {
//     todo!()
// }

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(room_create))
        .routes(routes!(room_get))
        .routes(routes!(room_list))
        .routes(routes!(room_edit))
    // .routes(routes!(room_ack))
    // .routes(routes!(dm_init))
    // .routes(routes!(dm_get))
}
