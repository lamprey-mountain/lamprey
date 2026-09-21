use std::time::Duration;

use crate::{
    endpoints::media::util::{MediaInfo, calculate_response_metadata},
    prelude::*,
    util::headers::{HeadersRequest, HeadersResponse},
};

use headers::CacheControl;
use http::StatusCode;
use routes::media_proxy as routes;

// PERF: cache files on local disk, similarly to how the s3 tantivy directory works
// either make cache_dir top level (usable by both search and media) or add another cache_dir to media config

#[handler(routes::media_head)]
async fn head(req: Req<routes::media_head::Endpoint>) -> Result<routes::media_head::Response> {
    let srv = req.services();
    let media_id = req.inner().media_id;
    let wait = req.inner().query.wait;

    let media = srv.media.get(media_id).await.cast_internal()?.ready().await;

    let meta = calculate_response_metadata(req.headers(), &MediaInfo::Media(&media))?;
    let status = meta.status();

    Ok(routes::media_head::Response {
        status,
        headers: meta.headers.into(),
    })
}

#[handler(routes::media_get)]
async fn get(req: Req<routes::media_get::Endpoint>) -> Result<routes::media_get::Response> {
    let srv = req.services();
    let media_id = req.inner().media_id;
    let wait = req.inner().query.wait;

    let media = srv.media.get(media_id).await.cast_internal()?.ready().await;

    let meta = calculate_response_metadata(req.headers(), &MediaInfo::Media(&media))?;
    let status = meta.status();

    Ok(routes::media_get::Response {
        status,
        headers: meta.headers.into(),
        body: todo!(),
    })
}

#[handler(routes::media_head_filename)]
async fn head_filename(
    req: Req<routes::media_head_filename::Endpoint>,
) -> Result<routes::media_head_filename::Response> {
    let srv = req.services();
    let media_id = req.inner().media_id;
    let filename = &req.inner().filename;
    let wait = req.inner().query.wait;

    let media = srv.media.get(media_id).await.cast_internal()?.ready().await;
    if &media.filename != filename {
        return Err(ApiError::from_code(ErrorCode::UnknownMedia).into());
    }

    let meta = calculate_response_metadata(req.headers(), &MediaInfo::Media(&media))?;
    let status = meta.status();

    Ok(routes::media_head_filename::Response {
        status,
        headers: meta.headers.into(),
    })
}

#[handler(routes::media_get_filename)]
async fn get_filename(
    req: Req<routes::media_get_filename::Endpoint>,
) -> Result<routes::media_get_filename::Response> {
    let srv = req.services();
    let media_id = req.inner().media_id;
    let filename = &req.inner().filename;
    let wait = req.inner().query.wait;

    let media = srv.media.get(media_id).await.cast_internal()?.ready().await;
    if &media.filename != filename {
        return Err(ApiError::from_code(ErrorCode::UnknownMedia).into());
    }

    let meta = calculate_response_metadata(req.headers(), &MediaInfo::Media(&media))?;
    let status = meta.status();

    Ok(routes::media_get_filename::Response {
        status,
        headers: meta.headers.into(),
        body: todo!(),
    })
}

export_routes!(head, get, head_filename, get_filename);
