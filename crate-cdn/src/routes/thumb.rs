use axum::{
    body::Body,
    extract::{Path, Query, State},
};
use common::v1::types::MediaId;
use http::{HeaderMap, StatusCode};
use image::codecs::avif::AvifEncoder;
use serde::Deserialize;
use std::io::Cursor;
use tracing::{error, span, Instrument, Level};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{
    data::{self},
    error::{Error, Result},
    routes::{
        media::{get_media, head_media},
        util::{build_thumb_headers_pre, complete_thumb_headers, get_thumb_source},
    },
    AppState,
};

#[derive(Deserialize)]
pub struct ThumbQuery {
    /// if None, fetch the original thumbnail (eg. a video may have an embedded thumbnail)
    pub size: Option<u32>,
}

/// Fetch thumbnail
///
/// get a thumbnail for a piece of media
#[utoipa::path(get, path = "/thumb/{media_id}")]
pub async fn get_thumb(
    State(state): State<AppState>,
    Path(media_id): Path<MediaId>,
    Query(query): Query<ThumbQuery>,
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    // NOTE: original thumbnails (eg. from videos) are already extracted and saved to /thumb/{media_id}/original
    if let Some(size) = query.size {
        if !state.config.thumb_sizes.contains(&size) {
            return Err(Error::BadRequest);
        }

        let media = data::lookup_media(&state.db, media_id).await?;
        let pre_header_info = build_thumb_headers_pre(&headers, &media)?;

        if pre_header_info.unmodified {
            return Ok((
                StatusCode::NOT_MODIFIED,
                pre_header_info.headers,
                Body::empty(),
            ));
        }

        let thumb_path = format!("/thumb/{media_id}/{size}x{size}");

        if state.s3.exists(&thumb_path).await? {
            let meta = state.s3.stat(&thumb_path).await?;
            let content_length = meta.content_length();
            let (headers, range) =
                complete_thumb_headers(&headers, &media, pre_header_info.headers, content_length)?;

            let reader = state.s3.reader(&thumb_path).await?;
            if let Some(r) = range {
                let body = Body::from_stream(reader.into_bytes_stream(r).await?);
                return Ok((StatusCode::PARTIAL_CONTENT, headers, body));
            } else {
                let body = Body::from_stream(reader.into_bytes_stream(..).await?);
                return Ok((StatusCode::OK, headers, body));
            }
        }

        // TODO: prevent races when generating thumbs
        // let thumb_lock = state
        //     .inflight
        //     .entry((media_id, size, size))
        //     .or_insert_with(|| Arc::new(Mutex::new(())))
        //     .clone();
        // let _guard = thumb_lock.lock().await;
        // // generate thumbnail...
        // drop(_guard);

        let Some(source_track) = get_thumb_source(&media) else {
            return Err(Error::NotFound);
        };

        let image_data = state
            .s3
            .read(source_track.url.path())
            .instrument(span!(Level::INFO, "read source media from s3"))
            .await?
            .to_bytes();
        let thumb_data = async {
            let image = image::load_from_memory(&image_data)?;
            let mut out = Cursor::new(Vec::new());
            let enc = AvifEncoder::new_with_speed_quality(&mut out, 4, 80);
            let thumb = image.thumbnail(size, size);
            thumb.write_with_encoder(enc)?;
            Result::Ok(out.into_inner())
        }
        .instrument(span!(Level::INFO, "generate thumbnail"))
        .await?;

        let s = state.s3.clone();
        let data_clone = thumb_data.clone();
        tokio::spawn(async move {
            if let Err(err) = s
                .write(&thumb_path, data_clone)
                .instrument(span!(Level::INFO, "upload thumbnail to s3"))
                .await
            {
                error!("error while uploading thumb: {err}")
            }
        });

        let (headers, _range) = complete_thumb_headers(
            &headers,
            &media,
            pre_header_info.headers,
            thumb_data.len() as u64,
        )?;

        Ok((StatusCode::OK, headers, Body::from(thumb_data)))
    } else {
        let media = data::lookup_media(&state.db, media_id).await?;
        let original_thumb_path = format!("/thumb/{media_id}/original");

        if state.s3.exists(&original_thumb_path).await? {
            let pre_header_info = build_thumb_headers_pre(&headers, &media)?;

            if pre_header_info.unmodified {
                return Ok((
                    StatusCode::NOT_MODIFIED,
                    pre_header_info.headers,
                    Body::empty(),
                ));
            }

            let meta = state.s3.stat(&original_thumb_path).await?;
            let content_length = meta.content_length();
            let (headers, range) =
                complete_thumb_headers(&headers, &media, pre_header_info.headers, content_length)?;

            let reader = state.s3.reader(&original_thumb_path).await?;
            if let Some(r) = range {
                let body = Body::from_stream(reader.into_bytes_stream(r).await?);
                return Ok((StatusCode::PARTIAL_CONTENT, headers, body));
            } else {
                let body = Body::from_stream(reader.into_bytes_stream(..).await?);
                return Ok((StatusCode::OK, headers, body));
            }
        }

        if media.source.mime.starts_with("image/") {
            return get_media(State(state), Path(media_id), headers).await;
        }

        Err(Error::NotFound)
    }
}

/// Head thumbnail
///
/// get headers for a thumbnail for a piece of media
#[utoipa::path(head, path = "/thumb/{media_id}")]
pub async fn head_thumb(
    State(state): State<AppState>,
    Path(media_id): Path<MediaId>,
    Query(query): Query<ThumbQuery>,
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    // NOTE: original thumbnails (eg. from videos) are already extracted and saved to /thumb/{media_id}/original
    if let Some(size) = query.size {
        if !state.config.thumb_sizes.contains(&size) {
            return Err(Error::BadRequest);
        }

        let media = data::lookup_media(&state.db, media_id).await?;
        let pre_header_info = build_thumb_headers_pre(&headers, &media)?;

        if pre_header_info.unmodified {
            return Ok((
                StatusCode::NOT_MODIFIED,
                pre_header_info.headers,
                Body::empty(),
            ));
        }

        let thumb_path = format!("/thumb/{media_id}/{size}x{size}");

        if state.s3.exists(&thumb_path).await? {
            let meta = state.s3.stat(&thumb_path).await?;
            let content_length = meta.content_length();
            let (headers, range) =
                complete_thumb_headers(&headers, &media, pre_header_info.headers, content_length)?;

            let status = if range.is_some() {
                StatusCode::PARTIAL_CONTENT
            } else {
                StatusCode::OK
            };
            return Ok((status, headers, Body::empty()));
        }

        // TODO: prevent races when generating thumbs
        // let thumb_lock = state
        //     .inflight
        //     .entry((media_id, size, size))
        //     .or_insert_with(|| Arc::new(Mutex::new(())))
        //     .clone();
        // let _guard = thumb_lock.lock().await;
        // // generate thumbnail...
        // drop(_guard);

        let Some(source_track) = get_thumb_source(&media) else {
            return Err(Error::NotFound);
        };

        let image_data = state
            .s3
            .read(source_track.url.path())
            .instrument(span!(Level::INFO, "read source media from s3"))
            .await?
            .to_bytes();
        let thumb_data = async {
            let image = image::load_from_memory(&image_data)?;
            let mut out = Cursor::new(Vec::new());
            let enc = AvifEncoder::new_with_speed_quality(&mut out, 4, 80);
            let thumb = image.thumbnail(size, size);
            thumb.write_with_encoder(enc)?;
            Result::Ok(out.into_inner())
        }
        .instrument(span!(Level::INFO, "generate thumbnail"))
        .await?;

        let s = state.s3.clone();
        let data_clone = thumb_data.clone();
        tokio::spawn(async move {
            if let Err(err) = s
                .write(&thumb_path, data_clone)
                .instrument(span!(Level::INFO, "upload thumbnail to s3"))
                .await
            {
                error!("error while uploading thumb: {err}")
            }
        });

        let (headers, range) = complete_thumb_headers(
            &headers,
            &media,
            pre_header_info.headers,
            thumb_data.len() as u64,
        )?;

        let status = if range.is_some() {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        };

        Ok((status, headers, Body::empty()))
    } else {
        let media = data::lookup_media(&state.db, media_id).await?;
        let original_thumb_path = format!("/thumb/{media_id}/original");

        if state.s3.exists(&original_thumb_path).await? {
            let pre_header_info = build_thumb_headers_pre(&headers, &media)?;

            if pre_header_info.unmodified {
                return Ok((
                    StatusCode::NOT_MODIFIED,
                    pre_header_info.headers,
                    Body::empty(),
                ));
            }

            let meta = state.s3.stat(&original_thumb_path).await?;
            let content_length = meta.content_length();
            let (headers, range) =
                complete_thumb_headers(&headers, &media, pre_header_info.headers, content_length)?;

            let status = if range.is_some() {
                StatusCode::PARTIAL_CONTENT
            } else {
                StatusCode::OK
            };
            return Ok((status, headers, Body::empty()));
        }

        if media.source.mime.starts_with("image/") {
            return head_media(State(state), Path(media_id), headers).await;
        }

        Err(Error::NotFound)
    }
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(head_thumb))
        .routes(routes!(get_thumb))
}
