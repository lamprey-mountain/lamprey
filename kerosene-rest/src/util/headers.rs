use std::{net::IpAddr, sync::Arc};

use crate::prelude::*;
use axum::{
    extract::FromRequestParts,
    response::{IntoResponseParts, ResponseParts},
};
use common::v1::types::{UserId, util::Time};
use headers::{
    AcceptRanges, Authorization, CacheControl, ContentDisposition, ContentLength, ETag,
    HeaderMapExt, IfMatch, IfModifiedSince, IfNoneMatch, IfRange, LastModified,
    authorization::Bearer,
};
use http::{HeaderMap, HeaderValue, request::Parts};
use kerosene_core::error::ErrorCode;

/// raw request headers for a request
// PERF: find some way to make this struct smaller?
#[derive(Debug, Default)]
pub struct HeadersRequest {
    /// authorization
    pub authorization: Option<Authorization<Bearer>>,

    /// x-reason
    pub reason: Option<String>,

    /// idempotency-key
    pub idempotency_key: Option<String>,

    /// x-puppet-id
    pub puppet_id: Option<UserId>,

    /// x-timestamp
    pub timestamp: Option<Time>,

    /// if-match
    pub if_match: Option<IfMatch>,

    /// if-none-match
    pub if_none_match: Option<IfNoneMatch>,

    /// if-modified-since
    pub if_modified_since: Option<IfModifiedSince>,

    /// if-range
    pub if_range: Option<IfRange>,

    /// content-type
    pub content_type: ContentType,

    /// user-agent
    pub user_agent: Option<String>,

    /// x-forwarded-for
    pub ip_addr: Option<IpAddr>,

    /// range
    pub range: Option<headers::Range>,
    // TODO: handle these headers
    // accept
    // accept-encoding (probably not, this should be handled by the reverse proxy?)
    // origin
}

/// raw response headers for a request
#[derive(Debug, Default)]
pub struct HeadersResponse {
    pub etag: Option<ETag>,
    pub last_modified: Option<LastModified>,
    pub accept_ranges: Option<AcceptRanges>,
    pub cache_control: Option<CacheControl>,
    pub content_disposition: Option<HeaderValue>,
    pub content_length: Option<ContentLength>,
    pub content_type: Option<headers::ContentType>,
    pub content_range: Option<headers::ContentRange>,
    // TODO: handle these headers
    // content-security-policy
    // permissions-policy
    // vary
}

/// parsed content type header
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// json body
    ///
    /// - `application/json`
    Json,

    /// form encoded body
    ///
    /// - `application/x-www-form-urlencoded`
    Form,

    /// msgpack request body
    ///
    /// - `application/vnd.msgpack`
    /// - `application/msgpack`
    /// - `application/x-msgpack`
    Msgpack,

    /// multipart request body
    ///
    /// - `multipart/form-data`
    Multipart,

    /// invalid or unknown content type
    Invalid,

    /// missing content type header
    #[default]
    Missing,
}

impl HeadersRequest {
    pub fn from_parts(parts: &Parts) -> Result<Self> {
        // FIXME: properly parse content-type; handle `application/foobar+json`
        let content_type = parts
            .headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .map(|s| {
                if s.starts_with("application/json") {
                    ContentType::Json
                } else if s.starts_with("application/x-www-form-urlencoded") {
                    ContentType::Form
                } else if s.starts_with("application/msgpack")
                    || s.starts_with("application/vnd.msgpack")
                    || s.starts_with("application/x-msgpack")
                {
                    ContentType::Msgpack
                } else if s.starts_with("multipart/form-data") {
                    ContentType::Multipart
                } else {
                    ContentType::Invalid
                }
            })
            .unwrap_or(ContentType::Missing);

        let reason = parts
            .headers
            .get("x-reason")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.to_string());

        if let Some(ref reason) = reason {
            if reason.chars().count() > 1024 {
                return Err(ApiError::with_message(
                    ErrorCode::BadHeader,
                    "X-Audit-Reason must be 1024 characters or less".to_string(),
                )
                .into());
            }
        }

        // TODO: handle x-real-ip

        // get the first ip address if they are comma separated
        let ip_addr = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim())
            .and_then(|s| s.parse().ok());

        Ok(Self {
            authorization: parts.headers.typed_get(),
            reason,
            idempotency_key: parts
                .headers
                .get("idempotency-key")
                .and_then(|h| h.to_str().ok())
                .map(|h| h.to_string()),
            puppet_id: parts
                .headers
                .get("x-puppet-id")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| h.parse().ok()),
            timestamp: parts
                .headers
                .get("x-timestamp")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| h.parse::<i64>().ok())
                .and_then(|secs| time::OffsetDateTime::from_unix_timestamp(secs).ok())
                .map(Time::from),
            if_match: parts.headers.typed_get(),
            if_none_match: parts.headers.typed_get(),
            if_modified_since: parts.headers.typed_get(),
            if_range: parts.headers.typed_get(),
            content_type,
            user_agent: parts
                .headers
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
            ip_addr,
            range: parts.headers.typed_get(),
        })
    }
}

impl IntoResponseParts for HeadersResponse {
    type Error = Error;

    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts> {
        if let Some(etag) = self.etag {
            res.headers_mut().typed_insert(etag);
        }
        if let Some(last_modified) = self.last_modified {
            res.headers_mut().typed_insert(last_modified);
        }
        if let Some(accept_ranges) = self.accept_ranges {
            res.headers_mut().typed_insert(accept_ranges);
        }
        if let Some(cache_control) = self.cache_control {
            res.headers_mut().typed_insert(cache_control);
        }
        if let Some(content_disposition) = self.content_disposition {
            res.headers_mut()
                .insert(http::header::CONTENT_DISPOSITION, content_disposition);
        }
        if let Some(content_length) = self.content_length {
            res.headers_mut().typed_insert(content_length);
        }
        if let Some(content_type) = self.content_type {
            res.headers_mut().typed_insert(content_type);
        }
        if let Some(content_range) = self.content_range {
            res.headers_mut().typed_insert(content_range);
        }
        Ok(res)
    }
}

impl From<HeadersResponse> for HeaderMap {
    fn from(value: HeadersResponse) -> Self {
        let mut headers = HeaderMap::new();
        if let Some(etag) = value.etag {
            headers.typed_insert(etag);
        }
        if let Some(last_modified) = value.last_modified {
            headers.typed_insert(last_modified);
        }
        if let Some(accept_ranges) = value.accept_ranges {
            headers.typed_insert(accept_ranges);
        }
        if let Some(cache_control) = value.cache_control {
            headers.typed_insert(cache_control);
        }
        if let Some(content_disposition) = value.content_disposition {
            headers.insert(http::header::CONTENT_DISPOSITION, content_disposition);
        }
        if let Some(content_length) = value.content_length {
            headers.typed_insert(content_length);
        }
        if let Some(content_type) = value.content_type {
            headers.typed_insert(content_type);
        }
        if let Some(content_range) = value.content_range {
            headers.typed_insert(content_range);
        }
        headers
    }
}
