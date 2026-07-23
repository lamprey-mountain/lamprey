use std::{net::IpAddr, sync::Arc};

use crate::ServerState;
use crate::prelude::*;
use axum::{
    extract::FromRequestParts,
    response::{IntoResponseParts, ResponseParts},
};
use common::v1::types::{UserId, util::Time};
use headers::Authorization;
use headers::authorization::Bearer;
use headers::{ETag, HeaderMapExt, IfMatch, IfModifiedSince, IfNoneMatch, LastModified};
use http::request::Parts;

pub struct HeadersRequest {
    pub authorization: Option<Authorization<Bearer>>,

    /// x-reason
    pub reason: Option<String>,

    pub idempotency_key: Option<String>,

    /// x-puppet-id
    pub puppet_id: Option<UserId>,

    /// x-timestamp
    pub timestamp: Option<Time>,

    pub if_match: Option<IfMatch>,
    pub if_none_match: Option<IfNoneMatch>,

    pub if_modified_since: Option<IfModifiedSince>,

    pub content_type: ContentType,

    /// user-agent
    pub user_agent: Option<String>,

    /// x-forwarded-for
    pub ip_addr: Option<IpAddr>,
}

pub struct HeadersResponse {
    pub etag: Option<ETag>,
    pub last_modified: Option<LastModified>,
}

/// parsed content type header
// TODO: maybe support application/x-www-form-urlencoded as well?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// json body
    ///
    /// - `application/json`
    Json,

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
                return Err(Error::BadRequest(
                    "X-Audit-Reason must be 1024 characters or less".to_string(),
                ));
            }
        }

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
            content_type,
            user_agent: parts
                .headers
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
            ip_addr,
        })
    }
}

impl FromRequestParts<Arc<ServerState>> for HeadersRequest {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &Arc<ServerState>) -> Result<Self> {
        Self::from_parts(parts)
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
        Ok(res)
    }
}
