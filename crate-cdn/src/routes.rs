use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use common::v1::types::{EmojiId, Media, MediaId, MediaTrack, MediaTrackInfo, TrackSource};
use headers::HeaderMapExt;
use http::{HeaderMap, StatusCode};
use image::codecs::avif::AvifEncoder;
use serde::Deserialize;
use std::{
    io::Cursor,
    ops::Bound,
    time::{Duration, SystemTime},
};
use tracing::{debug, error, span, Instrument, Level};
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
    let header_info = build_common_headers(&headers, &media)?;

    if header_info.unmodified {
        return Ok((StatusCode::NOT_MODIFIED, header_info.headers, Body::empty()));
    }

    let reader = state.s3.reader(&path).await?;
    if let Some(r) = header_info.range {
        let body = Body::from_stream(reader.into_bytes_stream(r).await?);
        Ok((StatusCode::PARTIAL_CONTENT, header_info.headers, body))
    } else {
        let body = Body::from_stream(reader.into_bytes_stream(..).await?);
        Ok((StatusCode::OK, header_info.headers, body))
    }
}

fn content_disposition_attachment(filename: &str, inline: bool) -> String {
    let a = if inline { "inline" } else { "attachment" };

    // For ASCII-only filenames, use simple format
    if filename.is_ascii() && !filename.contains(['\\', '/', '"']) {
        return format!("{a}; filename=\"{}\"", filename);
    }

    // For UTF-8 filenames, use RFC 6266 format with both parameters
    let ascii_fallback: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || ".-_ ".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();

    let encoded_filename =
        percent_encoding::utf8_percent_encode(filename, percent_encoding::NON_ALPHANUMERIC)
            .to_string();

    format!(
        "{a}; filename=\"{}\"; filename*=UTF-8''{}",
        ascii_fallback, encoded_filename
    )
}

struct HeaderInfo {
    headers: HeaderMap,
    range: Option<(Bound<u64>, Bound<u64>)>,
    unmodified: bool,
}

fn build_common_headers(req_headers: &HeaderMap, media: &Media) -> Result<HeaderInfo> {
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
        content_disposition_attachment(&media.filename, true)
            .parse()
            .unwrap(),
    );

    headers.typed_insert(
        headers::CacheControl::new()
            .with_public()
            .with_immutable()
            .with_max_age(Duration::from_secs(604800)),
    );

    let etag: headers::ETag = format!("W/\"{}\"", media.id).parse().unwrap();
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
                return Ok(HeaderInfo {
                    headers,
                    range: None,
                    unmodified: true,
                });
            }
        }

        if let Some(if_modified_since) = req_headers.typed_get::<headers::IfModifiedSince>() {
            if !if_modified_since.is_modified(id_timestamp) {
                return Ok(HeaderInfo {
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
            return Ok(HeaderInfo {
                headers,
                range: Some(range),
                unmodified: false,
            });
        }
    }
    headers.typed_insert(headers::ContentLength(content_length));
    Ok(HeaderInfo {
        headers,
        unmodified: false,
        range: None,
    })
}

// TODO: deduplicate this code
struct HeaderInfo2 {
    headers: HeaderMap,
    range: Option<(Bound<u64>, Bound<u64>)>,
    unmodified: bool,
    thumb: Option<MediaTrack>,
}

fn build_common_headers2(req_headers: &HeaderMap, media: &Media, size: u64) -> Result<HeaderInfo2> {
    let mut headers = HeaderMap::new();
    headers.typed_insert(headers::AcceptRanges::bytes());
    headers.typed_insert("image/avif".parse::<headers::ContentType>().unwrap());
    headers.insert(
        "content-disposition",
        // currently ALL thumbnails are avif
        // this will probably change in the future
        content_disposition_attachment("thumbnail.avif", true)
            .parse()
            .unwrap(),
    );

    headers.typed_insert(
        headers::CacheControl::new()
            .with_public()
            .with_immutable()
            .with_max_age(Duration::from_secs(604800)),
    );

    let etag: headers::ETag = format!("W/\"{}\"", media.id).parse().unwrap();
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
                return Ok(HeaderInfo2 {
                    headers,
                    range: None,
                    unmodified: true,
                    thumb: None,
                });
            }
        }

        if let Some(if_modified_since) = req_headers.typed_get::<headers::IfModifiedSince>() {
            if !if_modified_since.is_modified(id_timestamp) {
                return Ok(HeaderInfo2 {
                    headers,
                    range: None,
                    unmodified: true,
                    thumb: None,
                });
            }
        }

        true
    };

    // TODO: remove the following thumb selection code - i don't need it with a cdn!
    // i can always generate the right size thumbnail. i don't need to check tracks manually

    // direct port of the frontend algorithm: "get the largest image that fits in a w by h rect"
    // theres almost certainly a better way of doing this
    let mut thumbs: Vec<_> = media
        .all_tracks()
        .filter_map(|t| match &t.info {
            MediaTrackInfo::Thumbnail(i) => Some((t, i.width, i.height)),
            MediaTrackInfo::Image(i) => Some((t, i.width, i.height)),
            MediaTrackInfo::Mixed(i) => match (i.width, i.height) {
                (Some(w), Some(h)) => Some((t, w, h)),
                _ => None,
            },
            _ => None,
        })
        .collect();
    thumbs.sort_by(|(_, a, _), (_, b, _)| b.cmp(a));
    let thumb = thumbs
        .iter()
        .find_map(|(t, w, h)| {
            if *w <= size && *h <= size {
                Some(t)
            } else {
                None
            }
        })
        .ok_or(Error::NotFound)?;

    let content_length = thumb.size;
    if allow_range_requests {
        if let Some(ranges) = req_headers.typed_get::<headers::Range>() {
            let ranges: Vec<_> = ranges.satisfiable_ranges(content_length).collect();
            if ranges.len() != 1 {
                return Err(Error::BadRange);
            }
            let range = ranges[0];
            headers.typed_insert(headers::ContentRange::bytes(range, content_length).unwrap());
            return Ok(HeaderInfo2 {
                headers,
                range: Some(range),
                unmodified: false,
                thumb: Some(thumb.to_owned().to_owned()),
            });
        }
    }
    headers.typed_insert(headers::ContentLength(content_length));
    Ok(HeaderInfo2 {
        headers,
        unmodified: false,
        range: None,
        thumb: Some(thumb.to_owned().to_owned()),
    })
}

/// get the MediaTrack the thumbnail should be generated from
fn get_thumb_source(media: &Media) -> Option<&MediaTrack> {
    match &media.source.info {
        MediaTrackInfo::Image(_) | MediaTrackInfo::Thumbnail(_) => Some(&media.source),
        MediaTrackInfo::Mixed(m) if media.source.mime.starts_with("image/") => {
            match (m.width, m.height) {
                (Some(_), Some(_)) => Some(&media.source),
                _ => panic!("invalid data in db?"),
            }
        }
        _ => {
            if let Some(t) = media
                .all_tracks()
                .find(|t| t.source == TrackSource::Extracted && matches!(t.info, MediaTrackInfo::Thumbnail(_))) {
                Some(t)
            } else {
            media.all_tracks().find(|t| match &t.info {
                MediaTrackInfo::Thumbnail(_) => true,
                MediaTrackInfo::Image(_) => true,
                _ => false,
            })

            }

        }
    }
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
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    // TODO: save original thumbs (eg. from videos) to /thumb/{media-id}/original
    // TODO: return original thumbs if size is None. fallback to original image?

    let size = query.size.unwrap_or(64);
    if !state.config.thumb_sizes.contains(&size) {
        return Err(Error::BadRequest);
    }

    let media = data::lookup_media(&state.db, media_id).await?;
    let header_info = build_common_headers2(&headers, &media, size as u64)?;

    if header_info.unmodified {
        return Ok((StatusCode::NOT_MODIFIED, header_info.headers, Body::empty()));
    }

    let Some(thumb) = header_info.thumb else {
        debug!("no valid thumbnail for this image");
        // no valid thumbnail for this image
        return Ok((StatusCode::NOT_FOUND, header_info.headers, Body::empty()));
    };

    let thumb_path = thumb.url.path();
    if state.s3.exists(&thumb_path).await? {
        let reader = state.s3.reader(&thumb_path).await?;
        if let Some(r) = header_info.range {
            let body = Body::from_stream(reader.into_bytes_stream(r).await?);
            Ok((StatusCode::PARTIAL_CONTENT, header_info.headers, body))
        } else {
            let body = Body::from_stream(reader.into_bytes_stream(..).await?);
            Ok((StatusCode::OK, header_info.headers, body))
        }
    } else {
        // TODO: prevent races when generating thumbs
        // let thumb_lock = state
        //     .inflight
        //     .entry((media_id, size, size))
        //     .or_insert_with(|| Arc::new(Mutex::new(())))
        //     .clone();
        // let _guard = thumb_lock.lock().await;
        // // generate thumbnail...
        // drop(_guard);

        let image_data = state
            .s3
            .read(media.source.url.path())
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

        let a = thumb_path.to_owned();
        let b = thumb_data.clone();
        let s = state.s3.clone();
        tokio::spawn(async move {
            if let Err(err) = s
                .write(&a, b)
                .instrument(span!(Level::INFO, "upload thumbnail to s3"))
                .await
            {
                error!("error while uploading thumb: {err}")
            }
        });

        Ok((StatusCode::OK, header_info.headers, Body::from(thumb_data)))
    }
}

/// Fetch emoji
///
/// directly get an emoji's thumbnail
#[utoipa::path(get, path = "/emoji/{emoji_id}")]
async fn get_emoji(
    State(state): State<AppState>,
    Path(emoji_id): Path<EmojiId>,
    Query(query): Query<ThumbQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    // TODO: cache this lookup
    let media_id = lookup_emoji(&state.db, emoji_id).await?;
    get_thumb(State(state), Path(media_id), Query(query), headers).await
}

// fn get_thumb_pseudocode() {
//     let thumb_path = "thumb/{media_id}/{size}";
//     let media = get_media_from_db();

//     if file_exists(&thumb_path) {
//         // return the thumbnail
//         let reader = create_s3_reader(thumb_path);
//         return Ok(reader);
//     } else {
//         // generate a thumbnail
//         let Some(t) = get_thumb_source(media) else {
//             panic!("can't generate a thumbnail for this media");
//         };

//         let data = download(&t.url);
//         let thumb = generate_thumbnail(data);
//         upload_to_s3(thumb);
//         return Ok(thumb);
//     }
// }

// TODO: return better error messages (eg. in json)
pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        // TODO: http HEAD routes for media, thumb, emoji
        // .routes(routes!(head_media))
        .routes(routes!(get_media))
        .routes(routes!(get_thumb))
        .routes(routes!(get_emoji))
}
