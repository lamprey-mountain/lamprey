use axum::{
    body::Body,
    extract::{Path, Query, State},
};
use common::v1::types::MediaId;
use common::v2::types::media::proxy::{MediaQuery, ThumbQuery};
use futures_util::StreamExt;
use http::{HeaderMap, StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::error;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{
    error::{Error, Result},
    routes::{
        media::{get_media, head_media},
        util::{build_headers, probably_can_thumbnail, ContentInfo},
    },
    AppState,
};

// TODO: maybe allow generating png, jpeg, or webp thumbnails?
// NOTE: caniuse says avif has ~93% support
// NOTE: this may take up some extra space, should i impl thumbnail garbage collection? nah, probably not worth it
#[async_recursion::async_recursion]
async fn thumb_response(
    s: AppState,
    media_id: MediaId,
    query: ThumbQuery,
    media_query: MediaQuery,
    headers: HeaderMap,
    with_body: bool,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    s.ensure_media_ready(media_id, media_query.wait).await?;
    let animate = query.animate;
    if let Some(size) = query.size {
        if !s.config.thumb_sizes.contains(&size) {
            return Err(Error::BadRequest);
        }

        let media = s.lookup_media(media_id).await?;
        let pre_header_info = build_headers(
            &headers,
            &ContentInfo::Thumb {
                media: &media,
                content_length: None,
                animated: animate,
            },
        )?;

        if pre_header_info.unmodified {
            return Ok((
                StatusCode::NOT_MODIFIED,
                pre_header_info.headers,
                Body::empty(),
            ));
        }

        let ext = if animate { "webp" } else { "avif" };
        let suffix = if animate { "" } else { "_static" };
        let thumb_path = format!("/media/{media_id}/thumb/{size}x{size}{suffix}.{ext}");

        if s.s3.exists(&thumb_path).await? {
            let meta = s.s3.stat(&thumb_path).await?;
            let content_length = meta.content_length();
            let final_headers = build_headers(
                &headers,
                &ContentInfo::Thumb {
                    media: &media,
                    content_length: Some(content_length),
                    animated: animate,
                },
            )?;

            let status = if final_headers.range.is_some() {
                StatusCode::PARTIAL_CONTENT
            } else {
                StatusCode::OK
            };

            let body = if with_body {
                let reader = s.s3.reader(&thumb_path).await?;
                if let Some(r) = final_headers.range {
                    Body::from_stream(reader.into_bytes_stream(r).await?)
                } else {
                    Body::from_stream(reader.into_bytes_stream(..).await?)
                }
            } else {
                Body::empty()
            };

            return Ok((status, final_headers.headers, body));
        }

        let m = media.clone();
        let thumb_data = s
            .pending_thumbnails
            .try_get_with((media_id, size, size, animate), async move {
                let poster_path = format!("/media/{media_id}/poster");
                let source_path = if s.s3.exists(&poster_path).await? {
                    poster_path
                } else if probably_can_thumbnail(&m) {
                    format!("/media/{media_id}/file")
                } else {
                    return Err(Error::NotFound);
                };

                let temp_in = async_tempfile::TempFile::new().await?;
                let temp_out = async_tempfile::TempFile::new().await?;

                let reader = s.s3.reader(&source_path).await?;
                let mut writer = temp_in.open_rw().await?;
                let mut bytes_reader = reader.into_bytes_stream(..).await?;
                while let Some(chunk) = bytes_reader.next().await {
                    writer.write_all(&chunk?).await?;
                }
                writer.flush().await?;

                crate::ffmpeg::generate_thumbnail(
                    temp_in.file_path(),
                    temp_out.file_path(),
                    size,
                    animate,
                )
                .await?;

                let mut out_reader = temp_out.open_ro().await?;
                let mut thumb_data = Vec::new();
                out_reader.read_to_end(&mut thumb_data).await?;

                let s_clone = s.s3.clone();
                let data_clone = thumb_data.clone();
                tokio::spawn(async move {
                    if let Err(err) = s_clone.write(&thumb_path, data_clone).await {
                        error!("error while uploading thumb: {err}")
                    }
                });
                Ok(thumb_data)
            })
            .await?;

        let final_headers = build_headers(
            &headers,
            &ContentInfo::Thumb {
                media: &media,
                content_length: Some(thumb_data.len() as u64),
                animated: animate,
            },
        )?;

        let status = if final_headers.range.is_some() {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        };

        let body = if with_body {
            if let Some(range) = final_headers.range {
                let start = match range.0 {
                    std::ops::Bound::Included(s) => s,
                    std::ops::Bound::Excluded(s) => s + 1,
                    std::ops::Bound::Unbounded => 0,
                };
                let end = match range.1 {
                    std::ops::Bound::Included(e) => e + 1,
                    std::ops::Bound::Excluded(e) => e,
                    std::ops::Bound::Unbounded => thumb_data.len() as u64,
                };

                let part = thumb_data[start as usize..end as usize].to_vec();
                Body::from(part)
            } else {
                Body::from(thumb_data)
            }
        } else {
            Body::empty()
        };

        Ok((status, final_headers.headers, body))
    } else {
        let media = s.lookup_media(media_id).await?;

        let is_animated = media.source.mime.as_str() == "image/gif"
            || media.source.mime.as_str().starts_with("video/");

        if !animate && is_animated {
            // Force static thumbnail if animate=false is requested for an animated source
            let size = s.config.thumb_sizes.first().copied().unwrap_or(128);
            return thumb_response(
                s,
                media_id,
                ThumbQuery {
                    size: Some(size),
                    animate: false,
                },
                media_query,
                headers,
                with_body,
            )
            .await;
        }

        let poster_path = format!("/media/{media_id}/poster");

        if s.s3.exists(&poster_path).await? {
            let meta = s.s3.stat(&poster_path).await?;
            let content_length = meta.content_length();
            let final_headers = build_headers(
                &headers,
                &ContentInfo::Thumb {
                    media: &media,
                    content_length: Some(content_length),
                    animated: false,
                },
            )?;

            let status = if final_headers.range.is_some() {
                StatusCode::PARTIAL_CONTENT
            } else {
                StatusCode::OK
            };

            let body = if with_body {
                let reader = s.s3.reader(&poster_path).await?;
                if let Some(r) = final_headers.range {
                    Body::from_stream(reader.into_bytes_stream(r).await?)
                } else {
                    Body::from_stream(reader.into_bytes_stream(..).await?)
                }
            } else {
                Body::empty()
            };

            return Ok((status, final_headers.headers, body));
        }

        if media.source.mime.as_str().starts_with("image/") {
            if with_body {
                return get_media(
                    State(s.clone()),
                    Path(media_id),
                    Query(media_query),
                    headers,
                )
                .await;
            } else {
                return head_media(State(s), Path(media_id), Query(media_query), headers).await;
            }
        }

        Err(Error::NotFound)
    }
}

/// Fetch thumbnail
///
/// get a thumbnail for a piece of media
#[utoipa::path(get, path = "/thumb/{media_id}")]
pub async fn get_thumb(
    State(s): State<AppState>,
    Path(media_id): Path<MediaId>,
    Query(query): Query<ThumbQuery>,
    Query(media_query): Query<MediaQuery>,
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    thumb_response(s, media_id, query, media_query, headers, true).await
}

/// Head thumbnail
///
/// get headers for a thumbnail for a piece of media
#[utoipa::path(head, path = "/thumb/{media_id}")]
pub async fn head_thumb(
    State(s): State<AppState>,
    Path(media_id): Path<MediaId>,
    Query(query): Query<ThumbQuery>,
    Query(media_query): Query<MediaQuery>,
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    thumb_response(s, media_id, query, media_query, headers, false).await
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(head_thumb))
        .routes(routes!(get_thumb))
}
