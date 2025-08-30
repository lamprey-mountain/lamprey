use axum::{
    body::Body,
    extract::{Path, State},
};
use common::v1::types::MediaId;
use http::{HeaderMap, StatusCode};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{
    error::{Error, Result},
    routes::util::build_common_headers,
    AppState,
};

/// Head media
///
/// get headers for a piece of media
#[utoipa::path(head, path = "/media/{media_id}")]
pub async fn head_media(
    State(s): State<AppState>,
    Path(media_id): Path<MediaId>,
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    let media = s.lookup_media(media_id).await?;
    let header_info = build_common_headers(&headers, &media)?;

    if header_info.unmodified {
        return Ok((StatusCode::NOT_MODIFIED, header_info.headers, Body::empty()));
    }

    let status = if header_info.range.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };

    Ok((status, header_info.headers, Body::empty()))
}

/// Head media with filename
///
/// get headers for a piece of media
#[utoipa::path(head, path = "/media/{media_id}/{filename}")]
pub async fn head_media_filename(
    State(s): State<AppState>,
    Path((media_id, filename)): Path<(MediaId, String)>,
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    let media = s.lookup_media(media_id).await?;
    if media.filename != filename {
        return Err(Error::NotFound);
    }

    let header_info = build_common_headers(&headers, &media)?;

    if header_info.unmodified {
        return Ok((StatusCode::NOT_MODIFIED, header_info.headers, Body::empty()));
    }

    let status = if header_info.range.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };

    Ok((status, header_info.headers, Body::empty()))
}

/// Fetch media
///
/// download a piece of media
#[utoipa::path(get, path = "/media/{media_id}")]
pub async fn get_media(
    State(s): State<AppState>,
    Path(media_id): Path<MediaId>,
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    let path = format!("/media/{}", media_id);

    let media = s.lookup_media(media_id).await?;
    let header_info = build_common_headers(&headers, &media)?;

    if header_info.unmodified {
        return Ok((StatusCode::NOT_MODIFIED, header_info.headers, Body::empty()));
    }

    let reader = s.s3.reader(&path).await?;
    if let Some(r) = header_info.range {
        let body = Body::from_stream(reader.into_bytes_stream(r).await?);
        Ok((StatusCode::PARTIAL_CONTENT, header_info.headers, body))
    } else {
        let body = Body::from_stream(reader.into_bytes_stream(..).await?);
        Ok((StatusCode::OK, header_info.headers, body))
    }
}

/// Fetch media with filename
///
/// download a piece of media
#[utoipa::path(get, path = "/media/{media_id}/{filename}")]
pub async fn get_media_filename(
    State(state): State<AppState>,
    Path((media_id, filename)): Path<(MediaId, String)>,
    headers: HeaderMap,
    State(s): State<AppState>,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    let path = format!("/media/{}", media_id);

    let media = s.lookup_media(media_id).await?;
    if media.filename != filename {
        return Err(Error::NotFound);
    }

    let header_info = build_common_headers(&headers, &media)?;

    if header_info.unmodified {
        return Ok((StatusCode::NOT_MODIFIED, header_info.headers, Body::empty()));
    }

    let reader = state.s3.reader(&path).await?;
    if let Some(r) = header_info.range {
        let body = Body::from_stream(reader.into_bytes_stream(r).await?);
        Ok((StatusCode::PARTIAL_CONTENT, header_info.headers, body))
    } else {
        let body = Body::from_stream(reader.into_bytes_stream(..).await?);
        Ok((StatusCode::OK, header_info.headers, body))
    }
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(head_media_filename))
        .routes(routes!(head_media))
        .routes(routes!(get_media_filename))
        .routes(routes!(get_media))
}
