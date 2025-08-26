use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use bytes::Bytes;
use common::v1::types::{EmojiId, Media, MediaId};
use headers::HeaderMapExt;
use http::{HeaderMap, StatusCode};
use serde::Deserialize;
use std::{
    io::Cursor,
    ops::Bound,
    time::{Duration, SystemTime},
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{
    data::{self, lookup_emoji},
    error::{Error, Result},
    AppState,
};

/// Fetch media
///
/// download a piece of media
#[utoipa::path(get, path = "/media/{media_id}")]
async fn get_media(
    State(state): State<AppState>,
    Path(media_id): Path<MediaId>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let path = format!("/media/{}", media_id);

    let media = data::lookup_media(&state.db, media_id).await?;
    let asdf = build_common_headers(&headers, &media)?;

    if asdf.unmodified {
        return Ok((StatusCode::NOT_MODIFIED, asdf.headers, Bytes::new()));
    }

    let reader = state.s3.read_with(&path);
    if let Some(range) = asdf.range {
        Ok((
            StatusCode::PARTIAL_CONTENT,
            asdf.headers,
            reader.range(range).await?.to_bytes(),
        ))
    } else {
        Ok((StatusCode::OK, asdf.headers, reader.await?.to_bytes()))
    }
}

fn content_disposition_attachment(filename: &str) -> String {
    // For ASCII-only filenames, use simple format
    if filename.is_ascii() && !filename.contains(['\\', '"']) {
        return format!("attachment; filename=\"{}\"", filename);
    }

    // For UTF-8 filenames, use RFC 6266 format with both parameters
    let ascii_fallback = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || ".-_ ".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    let encoded_filename =
        percent_encoding::utf8_percent_encode(filename, percent_encoding::NON_ALPHANUMERIC)
            .to_string();

    format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{}",
        ascii_fallback, encoded_filename
    )
}

// TODO: come up with a good name for this
struct Asdf {
    headers: HeaderMap,
    range: Option<(Bound<u64>, Bound<u64>)>,
    unmodified: bool,
}

fn build_common_headers(req_headers: &HeaderMap, media: &Media) -> Result<Asdf> {
    let mut headers = HeaderMap::new();
    headers.typed_insert(headers::AcceptRanges::bytes());
    headers.typed_insert(
        media
            .source
            .mime
            .to_string()
            .parse::<headers::ContentType>()
            .unwrap(),
    );
    headers.insert(
        "content-disposition",
        content_disposition_attachment(&media.filename)
            .parse()
            .unwrap(),
    );

    headers.typed_insert(
        headers::CacheControl::new()
            .with_public()
            .with_immutable()
            .with_max_age(Duration::from_secs(604800)),
    );

    let etag = format!("W/\"{}\"", media.id).parse::<headers::ETag>().unwrap();
    headers.typed_insert(etag.clone());

    let id_timestamp: SystemTime = media
        .id
        .into_inner()
        .get_timestamp()
        .expect("all uuids are uuidv7")
        .into();
    let lm = headers::LastModified::from(id_timestamp);
    headers.typed_insert(lm);

    let allow_range_requests = if let Some(if_range) = headers.typed_get::<headers::IfRange>() {
        !if_range.is_modified(Some(&etag), Some(&lm))
    } else {
        if let Some(if_none_match) = req_headers.typed_get::<headers::IfNoneMatch>() {
            if !if_none_match.precondition_passes(&etag) {
                return Ok(Asdf {
                    headers,
                    range: None,
                    unmodified: true,
                });
            }
        }

        if let Some(if_modified_since) = req_headers.typed_get::<headers::IfModifiedSince>() {
            if !if_modified_since.is_modified(id_timestamp) {
                return Ok(Asdf {
                    headers,
                    range: None,
                    unmodified: true,
                });
            }
        }

        true
    };

    let content_length = media.source.size;
    if allow_range_requests {
        if let Some(ranges) = req_headers.typed_get::<headers::Range>() {
            let ranges: Vec<_> = ranges.satisfiable_ranges(content_length).collect();
            if ranges.len() != 1 {
                return Err(Error::BadRange);
            }
            let range = ranges[0];
            headers.typed_insert(headers::ContentRange::bytes(range, content_length).unwrap());
            return Ok(Asdf {
                headers,
                range: Some(range),
                unmodified: false,
            });
        }
    }
    headers.typed_insert(headers::ContentLength(content_length));
    Ok(Asdf {
        headers,
        unmodified: false,
        range: None,
    })
}

#[derive(Deserialize)]
struct ThumbQuery {
    /// if None, fetch the original thumbnail (eg. a video may have an embedded thumbnail)
    size: Option<u32>,
}

/// Fetch thumbnail
///
/// get a thumbnail for a piece of media
#[utoipa::path(get, path = "/thumb/{media_id}")]
async fn get_thumb(
    State(state): State<AppState>,
    Path(media_id): Path<MediaId>,
    Query(query): Query<ThumbQuery>,
) -> Result<impl IntoResponse> {
    let size = query.size.unwrap_or(64);
    if !state.config.thumb_sizes.contains(&size) {
        return Err(Error::BadRequest);
    }

    // AppState {
    //     inflight: Arc<DashMap<String, Arc<Mutex<()>>>>,
    // }
    // let key_lock = state
    //     .inflight
    //     .entry(key.clone())
    //     .or_insert_with(|| Arc::new(Mutex::new(())))
    //     .clone();
    // let _guard = key_lock.lock().await;
    // // generate thumbnail...
    // drop(_guard);

    let thumb_path = format!("/thumb/{}/{}x{}.webp", media_id, size, size);

    if state.s3.exists(&thumb_path).await.unwrap_or(false) {
        let data = state.s3.read(&thumb_path).await?;
        return Ok(data.to_bytes());
    }

    let media_path = format!("/media/{}", media_id);
    let media_data = state.s3.read(&media_path).await?.to_bytes();

    let image = image::load_from_memory(&media_data)?;
    let thumbnail = image.thumbnail(size, size);

    let mut buf = Cursor::new(Vec::new());
    thumbnail.write_to(&mut buf, image::ImageFormat::WebP)?;
    state
        .s3
        .write(&thumb_path, buf.clone().into_inner())
        .await?;

    Ok(Bytes::from(buf.into_inner()))
}

/// Fetch emoji
///
/// directly get an emoji's thumbnail
#[utoipa::path(get, path = "/emoji/{emoji_id}")]
async fn get_emoji(
    State(state): State<AppState>,
    Path(emoji_id): Path<EmojiId>,
    Query(query): Query<ThumbQuery>,
) -> Result<impl IntoResponse> {
    // TODO: cache
    let media_id = lookup_emoji(&state.db, emoji_id).await?;
    get_thumb(State(state), Path(media_id), Query(query)).await
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_media))
        .routes(routes!(get_thumb))
        .routes(routes!(get_emoji))
}
