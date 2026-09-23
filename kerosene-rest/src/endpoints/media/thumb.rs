use crate::{
    endpoints::media::util::{MediaInfo, calculate_response_metadata},
    prelude::*,
};
use common::util::body::Body;
use common::v1::types::MediaId;
use common::v2::types::media::proxy::ThumbQuery;
use http::StatusCode;
use kerosene_core::types::media::MediaPaths;
use routes::media_proxy as routes;

#[handler(routes::thumb_head)]
async fn head(req: Req<routes::thumb_head::Endpoint>) -> Result<routes::thumb_head::Response> {
    todo!()
}

#[handler(routes::thumb_get)]
async fn get(req: Req<routes::thumb_get::Endpoint>) -> Result<routes::thumb_get::Response> {
    let globals = req.globals();
    let srv = req.services();
    let media_id = req.inner().media_id;
    let query = &req.inner().query;

    let mut media_item = srv.media.get(media_id).await.cast_internal()?;
    let media = media_item.ready().await;

    let paths = MediaPaths::new("media/");
    let thumb_sizes = &globals.config().media.thumb_sizes;

    let (thumb_path, animated) = if let Some(size) = query.size {
        let size = if !thumb_sizes.contains(&size) {
            thumb_sizes
                .iter()
                .find(|&&s| s >= size)
                .copied()
                .or_else(|| thumb_sizes.last().copied())
                .ok_or_else(|| {
                    ApiError::with_message(
                        ErrorCode::InvalidData,
                        "thumbnail generation is disabled".to_string(),
                    )
                })?
        } else {
            size
        };

        media_item
            .generate_thumb(size, size, query.animate)
            .await
            .cast_internal()?;

        if query.animate {
            (paths.thumb(media.id, size, "webp"), true)
        } else {
            (paths.thumb_static(media.id, size, "avif"), false)
        }
    } else {
        (paths.poster(media.id), false)
    };

    let meta = calculate_response_metadata(
        req.headers(),
        &MediaInfo::Thumb {
            media: &media,
            content_length: None,
            animated,
        },
    )?;

    let body = if meta.unmodified {
        Body::empty()
    } else {
        // PERF: don't stat file every time
        let meta_data = globals
            .blobs()
            .stat(&thumb_path)
            .await
            .map_err(|_| ApiError::from_code(ErrorCode::NotFound))?;
        let content_length = meta_data.content_length();

        let meta = calculate_response_metadata(
            req.headers(),
            &MediaInfo::Thumb {
                media: &media,
                content_length: Some(content_length),
                animated,
            },
        )?;

        let reader = globals
            .blobs()
            .reader(&thumb_path)
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

    Ok(routes::thumb_get::Response {
        status: meta.status(),
        headers: meta.headers.into(),
        body,
    })
}

export_routes!(head, get);
