use std::{
    borrow::Cow,
    ops::Bound,
    time::{Duration, SystemTime},
};

use crate::{
    prelude::*,
    util::headers::{HeadersRequest, HeadersResponse},
};
use common::v2::types::media::Media;
use headers::{
    AcceptRanges, CacheControl, ContentDisposition, ContentLength, ContentRange, ContentType, ETag,
    LastModified,
};
use http::StatusCode;

// NOTE: renamed from ContentInfo
/// information about a piece of media
#[derive(Debug)]
pub enum MediaInfo<'a> {
    /// a piece of media directly
    Media(&'a Media),
    // TODO; support these
    // Thumb {
    //     media: &'a Media,
    //     content_length: Option<u64>,
    //     animated: bool,
    // },
    // Gifv {
    //     media: &'a Media,
    //     content_length: Option<u64>,
    // },
}

/// metadata needed to respond
#[derive(Debug)]
pub struct ResponseMetadata {
    pub headers: HeadersResponse,
    pub range: Option<(Bound<u64>, Bound<u64>)>,
    pub unmodified: bool,
}

impl<'a> MediaInfo<'a> {
    /// get the content type for this media
    pub fn content_type(&self) -> ContentType {
        match self {
            MediaInfo::Media(media) => media.content_type.to_string().parse().unwrap(),
        }
    }

    /// get the filename for this media
    pub fn filename(&self) -> Cow<'a, str> {
        match self {
            MediaInfo::Media(media) => Cow::Borrowed(&media.filename),
        }
    }

    /// get the length of this media
    pub fn len(&self) -> u64 {
        match self {
            MediaInfo::Media(media) => media.size,
        }
    }

    /// get the underlying media data
    pub fn media(&self) -> &'a Media {
        match self {
            MediaInfo::Media(media) => media,
        }
    }
}

impl ResponseMetadata {
    /// get the http status code that should be returned
    pub fn status(&self) -> StatusCode {
        if self.unmodified {
            StatusCode::NOT_MODIFIED
        } else if self.range.is_some() {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        }
    }
}

// NOTE: renamed from build_headers
pub fn calculate_response_metadata<'a>(
    headers_req: &HeadersRequest,
    info: &MediaInfo<'a>,
) -> Result<ResponseMetadata> {
    let media = info.media();
    let mut headers = HeadersResponse::default();

    // 1. caching headers
    headers.cache_control = Some(
        CacheControl::new()
            .with_public()
            .with_immutable()
            .with_max_age(Duration::from_secs(604800)),
    );

    let etag: ETag = format!("W/\"{}\"", media.id).parse().unwrap();
    headers.etag = Some(etag.clone());

    let id_timestamp: SystemTime = media
        .id
        .get_timestamp()
        .expect("all uuids are uuidv7")
        .into();
    let lm = LastModified::from(id_timestamp);
    headers.last_modified = Some(lm);

    // 2. content metadata headers
    headers.accept_ranges = Some(AcceptRanges::bytes());
    headers.content_type = Some(info.content_type());
    headers.content_disposition = Some(
        content_disposition_attachment(&info.filename(), true)
            .parse()
            .unwrap(),
    );

    // 3. check range request headers
    // if If-Range is present, return a range if content is not modified. otherwise, return the full content.
    if headers_req.if_range.is_none() {
        if let Some(if_none_match) = &headers_req.if_none_match {
            if !if_none_match.precondition_passes(&etag) {
                return Ok(ResponseMetadata {
                    headers,
                    range: None,
                    unmodified: true,
                });
            }
        }

        if let Some(if_modified_since) = &headers_req.if_modified_since {
            if !if_modified_since.is_modified(id_timestamp) {
                return Ok(ResponseMetadata {
                    headers,
                    range: None,
                    unmodified: true,
                });
            }
        }
    }

    // 4. try to insert Content-Length if we have it
    let content_length = info.len();
    let allow_range_request = if let Some(if_range) = &headers_req.if_range {
        !if_range.is_modified(Some(&etag), Some(&lm))
    } else {
        true
    };

    let mut range = None;
    if allow_range_request {
        if let Some(ranges) = &headers_req.range {
            let satisfiable_ranges: Vec<_> = ranges.satisfiable_ranges(content_length).collect();
            // TODO(future): handling multiple satisfiable_ranges?
            if satisfiable_ranges.len() != 1 {
                return Err(ApiError::with_message(
                    ErrorCode::InvalidData, // TODO: error code for invalid range
                    "Invalid range request".to_string(),
                )
                .into());
            }
            let r = satisfiable_ranges[0];
            headers.content_range = Some(ContentRange::bytes(r, content_length).unwrap());
            range = Some(r);
        }
    }

    if range.is_none() {
        headers.content_length = Some(ContentLength(content_length));
    }

    Ok(ResponseMetadata {
        headers,
        range,
        unmodified: false,
    })
}

// TODO: impl custom header for content disposition
// /// the http `Content-Disposition` header
// pub struct ContentDisposition {
//     filename: String,
//     inline: bool,
// }
//
// impl headers::Header for ContentDisposition {}

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
