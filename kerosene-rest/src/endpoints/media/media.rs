use std::time::Duration;

use crate::{
    endpoints::media::util::{MediaInfo, calculate_response_metadata},
    prelude::*,
    util::headers::{HeadersRequest, HeadersResponse},
};

use common::util::body::Body;
use headers::CacheControl;
use http::StatusCode;
use kerosene_core::types::media::MediaPaths;
use routes::media_proxy as routes;

// PERF: cache files on local disk, similarly to how the s3 tantivy directory works
// either make cache_dir top level (usable by both search and media) or add another cache_dir to media config

#[handler(routes::media_get)]
async fn get(req: Req<routes::media_get::Endpoint>) -> Result<routes::media_get::Response> {
    let globals = req.globals();
    let srv = req.services();
    let media_id = req.inner().media_id;
    let wait = req.inner().query.wait;

    let media = srv.media.get(media_id).await.cast_internal()?.ready().await;
    let meta = calculate_response_metadata(req.headers(), &MediaInfo::Media(&media))?;
    let body = if meta.unmodified || req.method() == http::Method::HEAD {
        Body::empty()
    } else {
        // TODO: better errors
        // PERF: cache MediaPaths
        let paths = MediaPaths::new("media/");
        let reader = globals
            .blobs()
            .reader(&paths.file(media.id))
            .await
            .map_err(|err| ServerError::Internal(Box::new(err)))?;
        let stream = if let Some(range) = meta.range {
            reader
                .into_bytes_stream(range)
                .await
                .map_err(|err| ServerError::Internal(Box::new(err)))?
        } else {
            reader
                .into_bytes_stream(..)
                .await
                .map_err(|err| ServerError::Internal(Box::new(err)))?
        };
        Body::from_stream(stream)
    };

    Ok(routes::media_get::Response {
        status: meta.status(),
        headers: meta.headers.into(),
        body,
    })
}

#[handler(routes::media_get_filename)]
async fn get_filename(
    req: Req<routes::media_get_filename::Endpoint>,
) -> Result<routes::media_get_filename::Response> {
    let globals = req.globals();
    let srv = req.services();
    let media_id = req.inner().media_id;
    let filename = &req.inner().filename;
    let wait = req.inner().query.wait;

    let media = srv.media.get(media_id).await.cast_internal()?.ready().await;
    if &media.filename != filename {
        return Err(ApiError::from_code(ErrorCode::UnknownMedia).into());
    }

    let meta = calculate_response_metadata(req.headers(), &MediaInfo::Media(&media))?;
    let body = if meta.unmodified || req.method() == http::Method::HEAD {
        Body::empty()
    } else {
        // TODO: better errors
        // PERF: cache MediaPaths
        let paths = MediaPaths::new("media/");
        let reader = globals
            .blobs()
            .reader(&paths.file(media.id))
            .await
            .map_err(|err| ServerError::Internal(Box::new(err)))?;
        let stream = if let Some(range) = meta.range {
            reader
                .into_bytes_stream(range)
                .await
                .map_err(|err| ServerError::Internal(Box::new(err)))?
        } else {
            reader
                .into_bytes_stream(..)
                .await
                .map_err(|err| ServerError::Internal(Box::new(err)))?
        };
        Body::from_stream(stream)
    };

    Ok(routes::media_get_filename::Response {
        status: meta.status(),
        headers: meta.headers.into(),
        body,
    })
}

export_routes!(get, get_filename);
