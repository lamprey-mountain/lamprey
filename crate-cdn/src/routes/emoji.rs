use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use common::v1::types::EmojiId;
use http::HeaderMap;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{
    data::lookup_emoji,
    error::Result,
    routes::thumb::{get_thumb, head_thumb, ThumbQuery},
    AppState,
};

/// Fetch emoji
///
/// directly get an emoji's thumbnail
#[utoipa::path(get, path = "/emoji/{emoji_id}")]
pub async fn get_emoji(
    State(state): State<AppState>,
    Path(emoji_id): Path<EmojiId>,
    Query(query): Query<ThumbQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    // TODO: cache this lookup
    let media_id = lookup_emoji(&state.db, emoji_id).await?;
    get_thumb(State(state), Path(media_id), Query(query), headers).await
}

/// Head emoji
///
/// directly get an emoji's thumbnail headers
#[utoipa::path(head, path = "/emoji/{emoji_id}")]
pub async fn head_emoji(
    State(state): State<AppState>,
    Path(emoji_id): Path<EmojiId>,
    Query(query): Query<ThumbQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    // TODO: cache this lookup
    let media_id = lookup_emoji(&state.db, emoji_id).await?;
    head_thumb(State(state), Path(media_id), Query(query), headers).await
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(head_emoji))
        .routes(routes!(get_emoji))
}
