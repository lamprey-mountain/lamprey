use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use common::v1::types::MediaId;
use http::StatusCode;
use serde::Deserialize;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{error::Result, AppState};

// NOTE: theres probably a better way to do this
#[derive(Debug, Deserialize)]
struct TrickplayQuery {
    /// number of thumbnails on the y axis
    pub height: Option<u32>,

    /// number of thumbnails on the x axis
    pub width: Option<u32>,

    /// height for each thumbnail
    pub thumb_height: Option<u32>,

    /// width for each thumbnail
    pub thumb_width: Option<u32>,
}

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
