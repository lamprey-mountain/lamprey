use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use common::{v1::types::MediaId, v2::types::media::proxy::TrickplayQuery};
use http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{error::Result, AppState};

/// Fetch trickplay (TODO)
#[utoipa::path(get, path = "/trickplay/{media_id}")]
async fn get_trickplay(
    State(_s): State<AppState>,
    Path(_media_id): Path<MediaId>,
    Query(_query): Query<TrickplayQuery>,
) -> Result<impl IntoResponse> {
    Ok((StatusCode::NOT_IMPLEMENTED, "doesn't exist yet :("))
}

/// Head trickplay (TODO)
#[utoipa::path(head, path = "/trickplay/{media_id}")]
async fn head_trickplay(
    State(_s): State<AppState>,
    Path(_media_id): Path<MediaId>,
    Query(_query): Query<TrickplayQuery>,
) -> Result<impl IntoResponse> {
    Ok((StatusCode::NOT_IMPLEMENTED, "doesn't exist yet :("))
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_trickplay))
        .routes(routes!(head_trickplay))
}
