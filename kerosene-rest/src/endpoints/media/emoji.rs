use crate::{
    endpoints::media::util::{MediaInfo, calculate_response_metadata},
    prelude::*,
};
use common::util::body::Body;
use kerosene_core::types::media::MediaPaths;
use routes::media_proxy as routes;

// TODO: deduplicate/dry with thumb.rs endpoint
#[handler(routes::emoji_get)]
async fn get(req: Req<routes::emoji_get::Endpoint>) -> Result<routes::emoji_get::Response> {
    let globals = req.globals();
    let srv = req.services();
    let emoji_id = req.inner().emoji_id;
    let query = &req.inner().query;

    let emoji = srv.emoji.get(emoji_id).await.cast_internal()?;
    let media_id = emoji.media_id;
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
        &if query.size.is_some() {
            MediaInfo::Thumb {
                media: &media,
                content_length: None,
                animated,
            }
        } else {
            MediaInfo::Media(&media)
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
            &if query.size.is_some() {
                MediaInfo::Thumb {
                    media: &media,
                    content_length: None,
                    animated,
                }
            } else {
                MediaInfo::Media(&media)
            },
        )?;

        if req.method() == http::Method::HEAD {
            Body::empty()
        } else {
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
        }
    };

    Ok(routes::emoji_get::Response {
        status: meta.status(),
        headers: meta.headers.into(),
        body,
    })
}

export_routes!(get);
