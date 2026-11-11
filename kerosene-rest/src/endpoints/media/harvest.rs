use std::time::SystemTime;

use crate::{
    endpoints::media::util::{MediaInfo, calculate_response_metadata},
    prelude::*,
};
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
        expires_at,
        completed_at,
        url,
        size,
        ..
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
        || token
            .as_bytes()
            .ct_eq(req.inner().token.as_bytes())
            .unwrap_u8()
            != 1
        || filename.is_empty()
        || filename.contains('/')
        || filename != req.inner().filename
    {
        return Err(unknown.into());
    }

    let info = MediaInfo::Harvest {
        id: harvest.id,
        filename: &req.inner().filename,
        content_length: size,
        completed_at: SystemTime::from(completed_at.into_inner()),
    };

    let meta = calculate_response_metadata(req.headers(), &info)?;

    let body = if meta.unmodified || req.method() == http::Method::HEAD {
        Body::empty()
    } else {
        let reader = globals
            .blobs()
            .reader(path)
            .await
            .map_err(|_| Error::from(unknown))?;
        let stream = if let Some(range) = meta.range {
            reader
                .into_bytes_stream(range)
                .await
                .map_err(|err| ServerError::internal(err))?
        } else {
            reader
                .into_bytes_stream(..)
                .await
                .map_err(|err| ServerError::internal(err))?
        };

        Body::from_stream(stream)
    };

    Ok(routes::harvest_get::Response {
        status: meta.status(),
        headers: meta.headers.into(),
        body,
    })
}

export_routes!(get);
