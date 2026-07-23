use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use axum::{
    extract::{FromRequest, FromRequestParts, Request, State},
    http::request::Parts,
};
use common::v1::types::{UserId, federation::Hostname, util::Time};
use common::{util::FederationBody, v1::types::headers::HEADER_ORIGIN};
use http::{HeaderMap, HeaderName, HeaderValue};
use kerosene_services::services::federation::signing::ValidatedKeyAlgo;
use serde::de::DeserializeOwned;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ServerState, error::Error};

pub mod audit;
pub mod auth;
pub mod auth_old; // TODO: remove
pub mod extract;
pub mod headers;
pub mod multipart;
pub mod script_http;
pub mod signing;

pub use audit::audit_log_middleware;
pub use auth_old::{Auth, Auth3, AuthRelaxed2};

// TODO: remove
/// extract the X-Reason header
pub struct HeaderReason(pub Option<String>);

// TODO: remove
/// extract the Idempotency-Key header
pub struct HeaderIdempotencyKey(pub Option<String>);

// TODO: remove
/// extract the X-Puppet-Id header
pub struct HeaderPuppetId(pub Option<UserId>);

// TODO: remove
/// extract the X-Timestamp header
pub struct HeaderTimestamp(pub Option<Time>);

// TODO: remove
/// extract caching http headers
pub struct HeaderCache {
    if_none_match: Option<HeaderValue>,
    if_modified_since: Option<HeaderValue>,
}

// TODO: move the below into a separate module
/// A verified federation identity
#[derive(Debug, Clone)]
pub struct FederationIdentity(pub Hostname);

/// validate a server request and extract json
#[derive(Clone)]
pub struct ServerAuth<T> {
    pub origin: Hostname,
    pub body: T,
}

pub async fn verify_server_request(
    state: &Arc<ServerState>,
    parts: &http::request::Parts,
    bytes: &::axum::body::Bytes,
) -> Result<Hostname, Error> {
    let method = parts.method.as_str();
    let path = parts.uri.path();
    let host_str = parts
        .headers
        .get(http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let origin_str = parts
        .headers
        .get(HEADER_ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let host = Hostname::new(host_str.to_string())?;
    let origin = Hostname::new(origin_str.to_string())?;

    let incoming = signing::IncomingRequest {
        origin: &origin,
        host: &host,
        method,
        path,
        body: bytes,
        headers: &parts.headers,
    };

    let srv = state.services();
    let keys = srv.federation.fetch_keys(&origin).await?;

    let mut verified = false;
    for key in &keys {
        let ValidatedKeyAlgo::Ed25519(verifying_key) = &key.alg;
        if incoming.verify(&verifying_key).is_ok() {
            verified = true;
            break;
        }
    }

    if !verified {
        return Err(Error::BadStatic(
            "no matching key found (possibly expired?)",
        ));
    }

    Ok(origin)
}

/// middleware to authenticate federation requests
pub async fn federation_auth_middleware(
    state: State<Arc<ServerState>>,
    req: Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, Error> {
    if !req.headers().contains_key(signing::HEADER_SIGNATURE) {
        return Ok(next.run(req).await);
    }

    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, 1024 * 1024 * 16)
        .await
        .map_err(|_| Error::BadStatic("failed to read body or body too large"))?;

    let origin = verify_server_request(&state, &parts, &bytes).await?;

    let mut req = Request::from_parts(parts, axum::body::Body::from(bytes.clone()));
    req.extensions_mut().insert(FederationIdentity(origin));
    req.extensions_mut().insert(FederationBody(bytes));

    Ok(next.run(req).await)
}

impl<T> FromRequest<Arc<ServerState>> for ServerAuth<T>
where
    T: DeserializeOwned,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &Arc<ServerState>) -> Result<Self, Self::Rejection> {
        if let Some(FederationBody(bytes)) = req.extensions().get::<FederationBody>() {
            let origin = req
                .extensions()
                .get::<FederationIdentity>()
                .cloned()
                .ok_or(Error::MissingAuth)?
                .0;
            let body: T = serde_json::from_slice(bytes)?;
            return Ok(ServerAuth { origin, body });
        }

        let (parts, body) = req.into_parts();
        let bytes = axum::body::to_bytes(body, 1024 * 1024 * 16)
            .await
            .map_err(|_| Error::BadStatic("failed to read body or body too large"))?;

        let origin = verify_server_request(state, &parts, &bytes).await?;
        let body: T = serde_json::from_slice(&bytes)?;

        Ok(ServerAuth { origin, body })
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderReason {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("X-Reason")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.to_string());

        if let Some(ref reason) = header {
            if reason.chars().count() > 1024 {
                return Err(Error::BadRequest(
                    "X-Audit-Reason must be 1024 characters or less".to_string(),
                ));
            }
        }

        Ok(Self(header))
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderIdempotencyKey {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("Idempotency-Key")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.to_string());
        Ok(Self(header))
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderPuppetId {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let puppet_id = parts
            .headers
            .get("X-Puppet-Id")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.parse().ok());
        Ok(Self(puppet_id))
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderTimestamp {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let timestamp = parts
            .headers
            .get("X-Timestamp")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.parse::<i64>().ok())
            .and_then(|secs| OffsetDateTime::from_unix_timestamp(secs).ok())
            .map(Time::from);
        Ok(Self(timestamp))
    }
}

impl HeaderCache {
    /// compare the etag of the request with the current etag
    fn compare_etag(&self, etag: &str) -> Result<(), Error> {
        if let Some(val) = &self.if_none_match {
            if val == etag {
                return Err(Error::NotModified);
            }
        }

        Ok(())
    }

    /// compare the last-modified-time of the request with the current mtime
    fn compare_mtime(&self, last_modified: &Time) -> Result<(), Error> {
        if let Some(val) = &self.if_modified_since {
            if let Ok(s) = val.to_str() {
                if let Ok(parsed_time) = httpdate::parse_http_date(s) {
                    let last_modified_st = SystemTime::UNIX_EPOCH
                        + Duration::from_secs(last_modified.unix_timestamp() as u64);

                    if last_modified_st <= parsed_time {
                        return Err(Error::NotModified);
                    }
                }
            }
        }
        Ok(())
    }

    /// compare version ids. returns the new caching headers
    pub fn compare_uuid(&self, uuid: &Uuid) -> Result<HeaderMap, Error> {
        let ts: Time = uuid
            .get_timestamp()
            .expect("this is a uuid v7")
            .try_into()
            .expect("uuids are always valid timestamps");
        let etag = format!(r#"W/"{}""#, uuid);
        self.compare_etag(&etag)?;
        self.compare_mtime(&ts)?;
        let headers = HeaderMap::from_iter([
            (
                HeaderName::from_static("last-modified"),
                HeaderValue::from_str(&httpdate::fmt_http_date(
                    (SystemTime::UNIX_EPOCH
                        + Duration::from_nanos(ts.unix_timestamp_nanos().try_into().unwrap_or(0)))
                    .into(),
                ))
                .unwrap(),
            ),
            (
                HeaderName::from_static("etag"),
                HeaderValue::from_str(&etag).unwrap(),
            ),
        ]);
        Ok(headers)
    }
}

impl FromRequestParts<Arc<ServerState>> for HeaderCache {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _s: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let if_none_match = parts.headers.get("if-none-match").cloned();
        let if_modified_since = parts.headers.get("if-modified-since").cloned();
        Ok(Self {
            if_none_match,
            if_modified_since,
        })
    }
}

#[macro_export]
macro_rules! routes2 {
    ($handler:ident) => {{
        type __PathStruct = pastey::paste! { [<$handler:camel Path>] };

        let path = <__PathStruct as ::utoipa::Path>::path();
        let methods = <__PathStruct as ::utoipa::Path>::methods();
        let operation = <__PathStruct as ::utoipa::Path>::operation();

        let schemas = ::std::vec![];

        let mut paths_builder = ::utoipa::openapi::path::PathsBuilder::new();
        for method in &methods {
            paths_builder = paths_builder.path(
                path.clone(),
                ::utoipa::openapi::PathItem::new(method.clone(), operation.clone()),
            );
        }
        let paths = paths_builder.build();

        let method_router =
            methods
                .iter()
                .fold(::axum::routing::MethodRouter::new(), |router, method| {
                    use ::utoipa_axum::PathItemExt as _;
                    router.on(method.to_method_filter(), $handler)
                });

        (schemas, paths, method_router)
    }};
}
