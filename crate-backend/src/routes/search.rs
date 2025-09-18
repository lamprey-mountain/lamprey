use std::sync::Arc;

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::search::{SearchMessageRequest, SearchRoomsRequest, SearchThreadsRequest};
use common::v1::types::{
    Message, MessageId, PaginationQuery, PaginationResponse, Room, RoomId, Thread, ThreadId,
};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::{Error, ServerState};

use super::util::Auth;
use crate::error::Result;

// maybe consider having one big search endgoint that searches *everything*?
// or maybe that's too expensive to do, idk

/// Search messages
#[utoipa::path(
    post,
    path = "/search/message",
    tags = ["search"],
    responses(
        (status = OK, body = PaginationResponse<Message>, description = "success"),
    )
)]
pub async fn search_messages(
    Auth(auth_user): Auth,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<PaginationQuery<MessageId>>,
    Json(json): Json<SearchMessageRequest>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let data = s.data();
    let mut res = data.search_message(auth_user.id, json, q).await?;
    for message in &mut res.items {
        s.presign_message(message).await?;
    }
    Ok(Json(res))
}

/// Search threads (TODO)
#[utoipa::path(
    post,
    path = "/search/thread",
    tags = ["search"],
    responses(
        (status = OK, body = PaginationResponse<Thread>, description = "success"),
    )
)]
pub async fn search_threads(
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Query(_q): Query<PaginationQuery<ThreadId>>,
    Json(_json): Json<SearchThreadsRequest>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Search rooms (TODO)
#[utoipa::path(
    post,
    path = "/search/room",
    tags = ["search"],
    responses(
        (status = OK, body = PaginationResponse<Room>, description = "success"),
    )
)]
#[axum::debug_handler]
pub async fn search_rooms(
    Auth(_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    Query(_q): Query<PaginationQuery<RoomId>>,
    Json(_json): Json<SearchRoomsRequest>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(search_messages))
        .routes(routes!(search_threads))
        .routes(routes!(search_rooms))
}
