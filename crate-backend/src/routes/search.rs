use std::sync::Arc;

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use types::{Message, MessageId, PaginationQuery, PaginationResponse, SearchMessageRequest};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use super::util::Auth;
use crate::error::Result;

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
    Auth(session, user_id): Auth,
    State(s): State<Arc<ServerState>>,
    Query(q): Query<PaginationQuery<MessageId>>,
    Json(body): Json<SearchMessageRequest>,
) -> Result<impl IntoResponse> {
    let data = s.data();
    let res = data.search_message(user_id, body, q).await?;
    Ok(Json(res))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes!(search_messages))
}
