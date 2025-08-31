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
    error::{Error, Result},
    routes::{
        media::{get_media, head_media},
        util::{build_headers, get_thumb_source, ContentInfo},
    },
    AppState,
};

#[derive(Deserialize)]
pub struct ThumbQuery {
    /// if None, fetch the original thumbnail (eg. a video may have an embedded thumbnail)
    pub size: Option<u32>,
}

// TODO: maybe allow generating png, jpeg, or webp thumbnails?
// NOTE: caniuse says avif has ~93% support
// NOTE: this may take up some extra space, should i impl thumbnail garbage collection? nah, probably not worth it
async fn thumb_response(
    s: AppState,
    media_id: MediaId,
    query: ThumbQuery,
    headers: HeaderMap,
    with_body: bool,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
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
            },
        )?;

        if pre_header_info.unmodified {
            return Ok((
                StatusCode::NOT_MODIFIED,
                pre_header_info.headers,
                Body::empty(),
            ));
        }

        let thumb_path = format!("/media/{media_id}/thumb/{size}x{size}.avif");

        if s.s3.exists(&thumb_path).await? {
            let meta = s.s3.stat(&thumb_path).await?;
            let content_length = meta.content_length();
            let final_headers = build_headers(
                &headers,
                &ContentInfo::Thumb {
                    media: &media,
                    content_length: Some(content_length),
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

        let Some(source_track) = get_thumb_source(&media) else {
            return Err(Error::NotFound);
        };

        let thumb_data = s
            .pending_thumbnails
            .try_get_with((media_id, size, size), async move {
                let image_data =
                    s.s3.read(source_track.url.path())
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

                let s_clone = s.s3.clone();
                let data_clone = thumb_data.clone();
                tokio::spawn(async move {
                    if let Err(err) = s_clone
                        .write(&thumb_path, data_clone)
                        .instrument(span!(Level::INFO, "upload thumbnail to s3"))
                        .await
                    {
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
        let poster_path = format!("/media/{media_id}/poster");

        if s.s3.exists(&poster_path).await? {
            let meta = s.s3.stat(&poster_path).await?;
            let content_length = meta.content_length();
            let final_headers = build_headers(
                &headers,
                &ContentInfo::Thumb {
                    media: &media,
                    content_length: Some(content_length),
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

        if media.source.mime.starts_with("image/") {
            if with_body {
                return get_media(State(s.clone()), Path(media_id), headers).await;
            } else {
                return head_media(State(s), Path(media_id), headers).await;
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
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    thumb_response(s, media_id, query, headers, true).await
}

/// Head thumbnail
///
/// get headers for a thumbnail for a piece of media
#[utoipa::path(head, path = "/thumb/{media_id}")]
pub async fn head_thumb(
    State(s): State<AppState>,
    Path(media_id): Path<MediaId>,
    Query(query): Query<ThumbQuery>,
    headers: HeaderMap,
) -> Result<(http::StatusCode, HeaderMap, Body)> {
    thumb_response(s, media_id, query, headers, false).await
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(head_thumb))
        .routes(routes!(get_thumb))
}
