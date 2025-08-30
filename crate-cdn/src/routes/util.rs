use common::v1::types::{Media, MediaTrack, MediaTrackInfo, TrackSource};
use headers::HeaderMapExt;
use http::HeaderMap;
use std::{
    ops::Bound,
    time::{Duration, SystemTime},
};

use crate::error::{Error, Result};

/// create a content-disposition header
pub fn content_disposition_attachment(filename: &str, inline: bool) -> String {
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

pub struct HeaderInfo {
    pub headers: HeaderMap,
    pub range: Option<(Bound<u64>, Bound<u64>)>,
    pub unmodified: bool,
}

pub fn build_common_headers(req_headers: &HeaderMap, media: &Media) -> Result<HeaderInfo> {
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

pub struct ThumbHeaderInfo {
    pub headers: HeaderMap,
    pub unmodified: bool,
}

pub fn build_thumb_headers_pre(req_headers: &HeaderMap, media: &Media) -> Result<ThumbHeaderInfo> {
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

    if let Some(if_none_match) = req_headers.typed_get::<headers::IfNoneMatch>() {
        if !if_none_match.precondition_passes(&etag) {
            return Ok(ThumbHeaderInfo {
                headers,
                unmodified: true,
            });
        }
    }

    if let Some(if_modified_since) = req_headers.typed_get::<headers::IfModifiedSince>() {
        if !if_modified_since.is_modified(id_timestamp) {
            return Ok(ThumbHeaderInfo {
                headers,
                unmodified: true,
            });
        }
    }

    Ok(ThumbHeaderInfo {
        headers,
        unmodified: false,
    })
}

pub fn complete_thumb_headers(
    req_headers: &HeaderMap,
    media: &Media,
    mut headers: HeaderMap,
    content_length: u64,
) -> Result<(HeaderMap, Option<(Bound<u64>, Bound<u64>)>)> {
    let etag: headers::ETag = format!("W/\"{}\"", media.id).parse().unwrap();
    let id_timestamp: SystemTime = media
        .id
        .into_inner()
        .get_timestamp()
        .expect("all uuids are uuidv7")
        .into();
    let lm = headers::LastModified::from(id_timestamp);

    let allow_range_requests = if let Some(if_range) = headers.typed_get::<headers::IfRange>() {
        !if_range.is_modified(Some(&etag), Some(&lm))
    } else {
        true
    };

    if allow_range_requests {
        if let Some(ranges) = req_headers.typed_get::<headers::Range>() {
            let ranges: Vec<_> = ranges.satisfiable_ranges(content_length).collect();
            if ranges.len() != 1 {
                return Err(Error::BadRange);
            }
            let range = ranges[0];
            headers.typed_insert(headers::ContentRange::bytes(range, content_length).unwrap());
            return Ok((headers, Some(range)));
        }
    }
    headers.typed_insert(headers::ContentLength(content_length));
    Ok((headers, None))
}

/// get the MediaTrack the thumbnail should be generated from
pub fn get_thumb_source(media: &Media) -> Option<&MediaTrack> {
    match &media.source.info {
        MediaTrackInfo::Image(_) | MediaTrackInfo::Thumbnail(_) => Some(&media.source),
        MediaTrackInfo::Mixed(m) if media.source.mime.starts_with("image/") => {
            match (m.width, m.height) {
                (Some(_), Some(_)) => Some(&media.source),
                _ => panic!("invalid data in db?"),
            }
        }
        _ => {
            if let Some(t) = media.all_tracks().find(|t| {
                t.source == TrackSource::Extracted && matches!(t.info, MediaTrackInfo::Thumbnail(_))
            }) {
                Some(t)
            } else {
                media.all_tracks().find(|t| {
                    matches!(
                        t.info,
                        MediaTrackInfo::Thumbnail(_) | MediaTrackInfo::Image(_)
                    )
                })
            }
        }
    }
}
