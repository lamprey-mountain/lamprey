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

pub struct BuiltHeaders {
    pub headers: HeaderMap,
    pub range: Option<(Bound<u64>, Bound<u64>)>,
    pub unmodified: bool,
}

pub enum ContentInfo<'a> {
    Media(&'a Media),
    Thumb {
        media: &'a Media,
        content_length: Option<u64>,
    },
}

impl<'a> ContentInfo<'a> {
    fn content_type(&self) -> headers::ContentType {
        match self {
            ContentInfo::Media(media) => media.source.mime.to_string().parse().unwrap(),
            ContentInfo::Thumb { .. } => "image/avif".parse().unwrap(),
        }
    }

    fn filename(&self) -> String {
        match self {
            ContentInfo::Media(media) => media.filename.clone(),
            ContentInfo::Thumb { .. } => "thumbnail.avif".to_string(),
        }
    }

    fn content_length(&self) -> Option<u64> {
        match self {
            ContentInfo::Media(media) => Some(media.source.size),
            ContentInfo::Thumb { content_length, .. } => *content_length,
        }
    }

    fn media(&self) -> &'a Media {
        match self {
            ContentInfo::Media(media) => media,
            ContentInfo::Thumb { media, .. } => media,
        }
    }
}

pub fn build_headers<'a>(
    req_headers: &HeaderMap,
    content_info: &ContentInfo<'a>,
) -> Result<BuiltHeaders> {
    let media = dbg!(content_info.media());
    let mut headers = HeaderMap::new();

    // step 1. generate and insert base headers
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
        .get_timestamp()
        .expect("all uuids are uuidv7")
        .into();
    let lm = headers::LastModified::from(id_timestamp);
    headers.typed_insert(lm);

    headers.typed_insert(headers::AcceptRanges::bytes());
    headers.typed_insert(dbg!(content_info.content_type()));
    headers.insert(
        "content-disposition",
        content_disposition_attachment(&content_info.filename(), true)
            .parse()
            .unwrap(),
    );

    // step 2. check range request headers
    // if If-Range is present, return a range if content is not modified. otherwise, return the full content.
    if req_headers.typed_get::<headers::IfRange>().is_none() {
        if let Some(if_none_match) = req_headers.typed_get::<headers::IfNoneMatch>() {
            if !if_none_match.precondition_passes(&etag) {
                return Ok(BuiltHeaders {
                    headers,
                    range: None,
                    unmodified: true,
                });
            }
        }

        if let Some(if_modified_since) = req_headers.typed_get::<headers::IfModifiedSince>() {
            if !if_modified_since.is_modified(id_timestamp) {
                return Ok(BuiltHeaders {
                    headers,
                    range: None,
                    unmodified: true,
                });
            }
        }
    }

    // step 3. try to insert Content-Length if we have it
    if let Some(content_length) = content_info.content_length() {
        let allow_range_request =
            if let Some(if_range) = req_headers.typed_get::<headers::IfRange>() {
                !if_range.is_modified(Some(&etag), Some(&lm))
            } else {
                true
            };

        let mut range = None;
        if allow_range_request {
            if let Some(ranges) = req_headers.typed_get::<headers::Range>() {
                let satisfiable_ranges: Vec<_> =
                    ranges.satisfiable_ranges(content_length).collect();
                if satisfiable_ranges.len() != 1 {
                    return Err(Error::BadRange);
                }
                let r = satisfiable_ranges[0];
                headers.typed_insert(headers::ContentRange::bytes(r, content_length).unwrap());
                range = Some(r);
            }
        }

        if range.is_none() {
            headers.typed_insert(headers::ContentLength(content_length));
        }

        Ok(BuiltHeaders {
            headers,
            range,
            unmodified: false,
        })
    } else {
        Ok(BuiltHeaders {
            headers,
            range: None,
            unmodified: false,
        })
    }
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
