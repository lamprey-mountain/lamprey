use axum::{extract::{Path, Query}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::{error::Error, types::{Membership, Permission, Room, RoomCreate, RoomId}, ServerState};

use super::util::{Auth, DatabaseConnection};

/// Create a room
#[utoipa::path(
    post,
    path = "/room",
    tags = ["room"],
)]
pub async fn room_create(
    Auth(session): Auth,
    DatabaseConnection(mut conn): DatabaseConnection,
    Json(json): Json<RoomCreate>,
) -> Result<(StatusCode, Json<Room>), Error> {
	let room_id = Uuid::now_v7();
	let user_id = session.user_id.into_inner();
	info!("got request");
	let mut tx = conn.begin().await?;
	let room = query_as!(Room, "
	    INSERT INTO room (id, version_id, name, description)
	    VALUES ($1, $2, $3, $4)
	    RETURNING id, version_id, name, description
    ", room_id, room_id, json.name, json.description)
	    .fetch_one(&mut *tx)
	    .await?;
	info!("inserted room");
	query!("
	    INSERT INTO room_member (user_id, room_id, membership)
	    VALUES ($1, $2, $3)
    ", user_id, room_id, Membership::Join as _)
	    .execute(&mut *tx)
	    .await?;
	info!("inserted member");
	let admin_role_id = Uuid::now_v7();
	query!(r#"
        INSERT INTO role (id, room_id, name, description, permissions, is_mentionable, is_self_applicable, is_default)
        VALUES ($1, $2, $3, $4, $5, false, false, false)
    "#, admin_role_id, room_id, "admin", Option::<String>::None, vec![Permission::Admin] as _)
	    .execute(&mut *tx)
    	.await?;
	info!("inserted role");
	query_as!(Role, r#"
        INSERT INTO role_member (user_id, role_id)
		VALUES ($1, $2)
    "#, user_id, admin_role_id)
	    .execute(&mut *tx)
    	.await?;
	info!("inserted role_member");
	tx.commit().await?;
    // events.emit("rooms", room.id, { type: "upsert.room", room });
	Ok((StatusCode::CREATED, Json(room)))
}

/// Get a room by its id.
#[utoipa::path(
    get,
    path = "/room/{id}",
    tags = ["room"],
    params(("id", description = "Room id")),
    responses(
        (status = 200, description = "Get room success", body = Room),
    )
)]
pub async fn room_get(
    Path((id, )): Path<(RoomId,)>,
    Auth(_session): Auth,
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<Json<Room>, Error> {
    let id: Uuid = id.into();
    let room = query_as!(Room, "SELECT id, version_id, name, description FROM room WHERE id = $1", id)
        .fetch_one(&mut *conn)
        .await?;
    Ok(Json(room))
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct PaginationQuery<I> {
    from: Option<I>,
    to: Option<I>,
    dir: PaginationDirection,
    limit: Option<u16>,
}

#[derive(Debug, Deserialize, Default, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PaginationDirection {
    #[default]
    F,
    B,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationResponse<T> {
    items: Vec<T>,
    total: u64,
    has_more: bool,
}

/// list visible rooms
#[utoipa::path(
    get,
    path = "/room",
    tags = ["room"],
    responses(
        (status = 200, description = "Paginate room success", body = PaginationResponse<Room>),
    )
)]
pub async fn room_list(
    Query(q): Query<PaginationQuery<RoomId>>,
    Auth(session): Auth,
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<Json<PaginationResponse<Room>>, Error> {
    if q.limit.is_some_and(|l| l > 100) {
        return Err(Error::TooBig);
    }
    let after = (if q.dir == PaginationDirection::F { q.from } else { q.to }).unwrap_or(RoomId(Uuid::nil()));
    let before = (if q.dir == PaginationDirection::F { q.to } else { q.from }).unwrap_or(RoomId(Uuid::max()));
    let limit = q.limit.unwrap_or(10);
    let mut tx = conn.begin().await?;
    let rooms = query_as!(Room, "
    	SELECT room.id, room.version_id, room.name, room.description FROM room_member
    	JOIN room ON room_member.room_id = room.id
    	WHERE room_member.user_id = $1 AND room.id > $2 AND room.id < $3
    	ORDER BY (CASE WHEN $4 = 'f' THEN room.id END), room.id DESC LIMIT $5
    ", session.user_id.into_inner(), after.into_inner(), before.into_inner(), if q.dir == PaginationDirection::F { "f" } else { "b" }, (limit + 1) as i32)
    .fetch_all(&mut *tx).await?;
    let total = query_scalar!("SELECT count(*) FROM room_member WHERE room_member.user_id = $1", session.user_id.into_inner())
        .fetch_one(&mut *tx).await?;
    tx.rollback().await?;
	let has_more = rooms.len() > limit as usize;
    let mut items: Vec<_> = rooms.into_iter().take(limit as usize).collect();
    if q.dir == PaginationDirection::B {
        items.reverse();
    }
	Ok(Json(PaginationResponse {
	    items,
	    total: total.unwrap_or(0) as u64,
	    has_more,
	}))
}

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        .routes(routes!(room_create))
        .routes(routes!(room_get))
        .routes(routes!(room_list))
}
