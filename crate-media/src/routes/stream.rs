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

#[derive(Debug, Deserialize)]
struct StreamQuery {
    /// segment index
    pub n: usize,

    /// stream identifier
    pub s: u64,
}

// TODO: move to common?
/// an available stream format
#[derive(Debug, Deserialize)]
pub struct StreamFormat {
    id: u64,

    kind: StreamKind,
    codec: String,

    width: Option<u64>,     // video only
    height: Option<u64>,    // video only
    framerate: Option<u64>, // video only

    bitrate: Option<u64>,  // audio only
    channels: Option<u64>, // audio only
}

#[derive(Debug, Deserialize)]
enum StreamKind {
    Video,
    Audio,
}

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
