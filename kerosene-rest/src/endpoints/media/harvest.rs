use crate::prelude::*;
use common::util::body::Body;
use common::v1::types::harvest::HarvestStatus;
use common::v1::types::util::Time;
use routes::media_proxy as routes;
use subtle::ConstantTimeEq as _;

#[handler(routes::harvest_get)]
async fn get(req: Req<routes::harvest_get::Endpoint>) -> Result<routes::harvest_get::Response> {
    let globals = req.globals();

    let unknown = ApiError::from_code(ErrorCode::UnknownHarvest);

    let harvest = globals
        .begin_read()
        .await
        .cast_internal()?
        .harvest_get(req.inner().harvest_id)
        .await
        .cast_internal()?
        .ok_or_else(|| Error::from(unknown.clone()))?;

    let HarvestStatus::Completed {
        expires_at, url, ..
    } = harvest.status
    else {
        return Err(unknown.into());
    };
    if Time::now_utc() >= expires_at {
        return Err(unknown.into());
    }

    // the access token and filename are embedded in the download url path
    //
    // expecting `harvest/{harvest_id}/{token}/{filename}`
    let path = url.path().trim_start_matches('/');
    let mut segments = path.splitn(4, '/');
    let (Some("harvest"), Some(id), Some(token), Some(filename)) = (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) else {
        return Err(unknown.into());
    };

    if id != harvest.id.to_string()
        || token.is_empty()
        || token.as_bytes().ct_eq(req.inner().token.as_bytes()).unwrap_u8() != 1
        || filename.is_empty()
        || filename.contains('/')
        || filename != req.inner().filename
    {
        return Err(unknown.into());
    }

    let reader = globals
        .blobs()
        .reader(path)
        .await
        .map_err(|_| Error::from(unknown))?;
    let stream = reader
        .into_bytes_stream(..)
        .await
        .map_err(|err| ServerError::internal(err))?;

    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("application/zip"),
    );
    headers.insert(
        http::header::CACHE_CONTROL,
        http::HeaderValue::from_static("private, no-store"),
    );
    headers.insert(
        http::header::CONTENT_DISPOSITION,
        http::HeaderValue::from_str(&format!(
            "attachment; filename=\"{}\"",
            req.inner().filename
        ))
        .map_err(|err| ServerError::internal(err))?,
    );

    Ok(routes::harvest_get::Response {
        status: http::StatusCode::OK,
        headers,
        body: Body::from_stream(stream),
    })
}

export_routes!(get);
