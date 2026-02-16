use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use common::{v1::types::MediaId, v2::types::media::proxy::StreamQuery};
use http::StatusCode;
use serde::Deserialize;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{error::Result, AppState};

/// Fetch stream (TODO)
#[utoipa::path(get, path = "/stream/{media_id}")]
async fn get_stream(
    State(_s): State<AppState>,
    Path(_media_id): Path<MediaId>,
    Query(_query): Query<StreamQuery>,
) -> Result<impl IntoResponse> {
    Ok((StatusCode::NOT_IMPLEMENTED, "doesn't exist yet :("))
}

/// Head stream (TODO)
#[utoipa::path(head, path = "/stream/{media_id}")]
async fn head_stream(
    State(_s): State<AppState>,
    Path(_media_id): Path<MediaId>,
    Query(_query): Query<StreamQuery>,
) -> Result<impl IntoResponse> {
    Ok((StatusCode::NOT_IMPLEMENTED, "doesn't exist yet :("))
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_stream))
        .routes(routes!(head_stream))
}
