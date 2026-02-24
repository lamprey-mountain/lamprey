use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use common::{
    v1::types::MediaId,
    v2::types::media::proxy::{MediaQuery, StreamQuery},
};
use http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{error::Result, AppState};

/// Fetch stream (TODO)
#[utoipa::path(get, path = "/stream/{media_id}")]
async fn get_stream(
    State(s): State<AppState>,
    Path(media_id): Path<MediaId>,
    Query(_query): Query<StreamQuery>,
    Query(media_query): Query<MediaQuery>,
) -> Result<impl IntoResponse> {
    s.ensure_media_ready(media_id, media_query.wait).await?;
    Ok((StatusCode::NOT_IMPLEMENTED, "doesn't exist yet :("))
}

/// Head stream (TODO)
#[utoipa::path(head, path = "/stream/{media_id}")]
async fn head_stream(
    State(s): State<AppState>,
    Path(media_id): Path<MediaId>,
    Query(_query): Query<StreamQuery>,
    Query(media_query): Query<MediaQuery>,
) -> Result<impl IntoResponse> {
    s.ensure_media_ready(media_id, media_query.wait).await?;
    Ok((StatusCode::NOT_IMPLEMENTED, "doesn't exist yet :("))
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_stream))
        .routes(routes!(head_stream))
}
