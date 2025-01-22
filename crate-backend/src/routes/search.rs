use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use serde::Deserialize;
use types::{MessageId, MessageServer, PaginationQuery, RoomId, SearchMessageRequest, UserCreateRequest, UserId, UserPatch};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::types::{UserCreate, UserIdReq};
use crate::ServerState;

use crate::error::{Error, Result};
use super::util::Auth;

/// Search messages
#[utoipa::path(
    post,
    path = "/search/message",
    tags = ["search"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn search_messages(
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<PaginationQuery<MessageId>>,
    Json(body): Json<SearchMessageRequest>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let res = data.search_message(session.user_id, body, q).await?;
    Ok(Json(res))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(search_messages))
}
